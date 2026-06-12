//! Aho-Corasick prefiltered model route resolver with glob-backed final matching.

use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt::{self, Display, Formatter},
};

use aho_corasick::{AhoCorasick, AhoCorasickBuilder};
use globset::{Glob, GlobSet, GlobSetBuilder};
use marlin_agent_protocol::{
    ModelCommandMatcher, ModelRouteDecision, ModelRouteReceipt, ModelRouteRequest, ModelRouteRule,
};

/// Compiled route resolver backed by Aho-Corasick prefilters and `globset`.
#[derive(Clone, Debug)]
pub struct CompiledModelRouteResolver {
    rules: Vec<CompiledModelRouteRule>,
    literal_index: CompiledRouteLiteralIndex,
}

impl CompiledModelRouteResolver {
    pub fn new(rules: Vec<ModelRouteRule>) -> Result<Self, ModelRouteCompileError> {
        let mut compiled = rules
            .into_iter()
            .enumerate()
            .map(|(index, rule)| CompiledModelRouteRule::compile(index, rule))
            .collect::<Result<Vec<_>, _>>()?;
        compiled.sort_by(|left, right| {
            right
                .rule
                .priority
                .cmp(&left.rule.priority)
                .then(left.index.cmp(&right.index))
        });
        let literal_index = CompiledRouteLiteralIndex::compile(&compiled)?;
        Ok(Self {
            rules: compiled,
            literal_index,
        })
    }

    pub fn resolve(&self, request: &ModelRouteRequest) -> Option<ModelRouteDecision> {
        let haystack = route_literal_haystack(request);
        self.literal_index
            .candidate_rule_indexes(haystack.as_str())
            .into_iter()
            .find_map(|index| self.rules[index].resolve(request))
    }
}

#[derive(Clone, Debug)]
struct CompiledModelRouteRule {
    index: usize,
    rule: ModelRouteRule,
    matcher: CompiledModelCommandMatcher,
}

impl CompiledModelRouteRule {
    fn compile(index: usize, rule: ModelRouteRule) -> Result<Self, ModelRouteCompileError> {
        let matcher = CompiledModelCommandMatcher::compile(&rule.matcher)?;
        Ok(Self {
            index,
            rule,
            matcher,
        })
    }

    fn resolve(&self, request: &ModelRouteRequest) -> Option<ModelRouteDecision> {
        let matched_globs = self.matcher.matches(request)?;
        let receipt = ModelRouteReceipt {
            rule_id: self.rule.rule_id.clone(),
            matched_globs,
            command_line: request.command_line(),
            litellm_model_id: self.rule.endpoint.litellm_model_id(),
            session_lifecycle: self.rule.session.lifecycle.clone(),
            context_fork: self.rule.session.context.clone(),
            requested_session_id: self.rule.session.requested_session_id.clone(),
            agent_scope: request.agent_scope.clone(),
            environment_activation: None,
            fallback_reason: None,
        };

        Some(ModelRouteDecision {
            endpoint: self.rule.endpoint.clone(),
            session: self.rule.session.clone(),
            receipt,
        })
    }

    fn literal_anchors(&self) -> Vec<String> {
        self.matcher.literal_anchors()
    }
}

#[derive(Clone, Debug)]
struct CompiledModelCommandMatcher {
    executable: CompiledGlobDimension,
    argv: CompiledGlobDimension,
    cwd: CompiledGlobDimension,
    workspace: CompiledGlobDimension,
    sub_agent_role: CompiledGlobDimension,
    agent_scope: CompiledGlobDimension,
    command_kind: CompiledGlobDimension,
}

impl CompiledModelCommandMatcher {
    fn compile(matcher: &ModelCommandMatcher) -> Result<Self, ModelRouteCompileError> {
        Ok(Self {
            executable: CompiledGlobDimension::compile("executable", &matcher.executable_globs)?,
            argv: CompiledGlobDimension::compile("argv", &matcher.argv_globs)?,
            cwd: CompiledGlobDimension::compile("cwd", &matcher.cwd_globs)?,
            workspace: CompiledGlobDimension::compile("workspace", &matcher.workspace_globs)?,
            sub_agent_role: CompiledGlobDimension::compile(
                "sub_agent_role",
                &matcher.sub_agent_role_globs,
            )?,
            agent_scope: CompiledGlobDimension::compile("agent_scope", &matcher.agent_scope_globs)?,
            command_kind: CompiledGlobDimension::compile(
                "command_kind",
                &matcher.command_kind_globs,
            )?,
        })
    }

    fn matches(&self, request: &ModelRouteRequest) -> Option<Vec<String>> {
        let mut matched = Vec::new();
        self.executable
            .matches(request.executable.as_deref(), &mut matched)?;
        let command_line = request.command_line();
        self.argv
            .matches(Some(command_line.as_str()), &mut matched)?;
        self.cwd.matches(request.cwd.as_deref(), &mut matched)?;
        self.workspace
            .matches(request.workspace.as_deref(), &mut matched)?;
        self.sub_agent_role
            .matches(request.sub_agent_role.as_deref(), &mut matched)?;
        self.agent_scope.matches(
            request.agent_scope.as_ref().map(|scope| scope.as_str()),
            &mut matched,
        )?;
        self.command_kind.matches(
            request.command_kind.as_ref().map(|kind| kind.as_str()),
            &mut matched,
        )?;
        Some(matched)
    }

    fn literal_anchors(&self) -> Vec<String> {
        let mut anchors = BTreeSet::new();
        self.executable.append_literal_anchors(&mut anchors);
        self.argv.append_literal_anchors(&mut anchors);
        self.cwd.append_literal_anchors(&mut anchors);
        self.workspace.append_literal_anchors(&mut anchors);
        self.sub_agent_role.append_literal_anchors(&mut anchors);
        self.agent_scope.append_literal_anchors(&mut anchors);
        self.command_kind.append_literal_anchors(&mut anchors);
        anchors.into_iter().collect()
    }
}

#[derive(Clone, Debug)]
struct CompiledGlobDimension {
    label: &'static str,
    patterns: Vec<String>,
    literal_prefilter: Option<CompiledLiteralPrefilter>,
    globset: Option<GlobSet>,
}

impl CompiledGlobDimension {
    fn compile(label: &'static str, patterns: &[String]) -> Result<Self, ModelRouteCompileError> {
        if patterns.is_empty() {
            return Ok(Self {
                label,
                patterns: Vec::new(),
                literal_prefilter: None,
                globset: None,
            });
        }

        let literal_prefilter = CompiledLiteralPrefilter::compile(label, patterns)?;
        let mut builder = GlobSetBuilder::new();
        for pattern in patterns {
            let glob = Glob::new(pattern)
                .map_err(|source| ModelRouteCompileError::new(label, pattern.clone(), source))?;
            builder.add(glob);
        }
        let globset = builder
            .build()
            .map_err(|source| ModelRouteCompileError::new(label, patterns.join("|"), source))?;

        Ok(Self {
            label,
            patterns: patterns.to_vec(),
            literal_prefilter,
            globset: Some(globset),
        })
    }

    fn matches(&self, value: Option<&str>, matched: &mut Vec<String>) -> Option<()> {
        let Some(globset) = &self.globset else {
            return Some(());
        };
        let value = value?;
        if self
            .literal_prefilter
            .as_ref()
            .is_some_and(|prefilter| prefilter.rejects(value))
        {
            return None;
        }
        let indexes = globset.matches(value);
        if indexes.is_empty() {
            return None;
        }
        matched.extend(
            indexes
                .into_iter()
                .map(|index| format!("{}:{}", self.label, self.patterns[index])),
        );
        Some(())
    }

    fn append_literal_anchors(&self, anchors: &mut BTreeSet<String>) {
        if let Some(prefilter) = &self.literal_prefilter {
            anchors.extend(prefilter.anchors().iter().cloned());
        }
    }
}

#[derive(Clone, Debug)]
struct CompiledLiteralPrefilter {
    anchors: Vec<String>,
    automaton: AhoCorasick,
    can_reject: bool,
}

impl CompiledLiteralPrefilter {
    fn compile(
        label: &'static str,
        patterns: &[String],
    ) -> Result<Option<Self>, ModelRouteCompileError> {
        let anchors = patterns
            .iter()
            .filter_map(|pattern| glob_literal_anchor(pattern))
            .collect::<Vec<_>>();
        let has_unanchored_pattern = anchors.len() != patterns.len();

        if anchors.is_empty() {
            return Ok(None);
        }

        let automaton = AhoCorasickBuilder::new()
            .ascii_case_insensitive(false)
            .build(&anchors)
            .map_err(|source| ModelRouteCompileError::new(label, anchors.join("|"), source))?;
        Ok(Some(Self {
            anchors,
            automaton,
            can_reject: !has_unanchored_pattern,
        }))
    }

    fn anchors(&self) -> &[String] {
        &self.anchors
    }

    fn rejects(&self, value: &str) -> bool {
        self.can_reject && !self.automaton.is_match(value)
    }
}

#[derive(Clone, Debug)]
struct CompiledRouteLiteralIndex {
    automaton: Option<AhoCorasick>,
    rules_by_pattern: Vec<Vec<usize>>,
    unindexed_rule_indexes: Vec<usize>,
}

impl CompiledRouteLiteralIndex {
    fn compile(rules: &[CompiledModelRouteRule]) -> Result<Self, ModelRouteCompileError> {
        let (rules_by_anchor, unindexed_rule_indexes) = route_literal_anchor_index(rules);

        if rules_by_anchor.is_empty() {
            return Ok(Self {
                automaton: None,
                rules_by_pattern: Vec::new(),
                unindexed_rule_indexes,
            });
        }

        let patterns = rules_by_anchor.keys().cloned().collect::<Vec<_>>();
        let rules_by_pattern = patterns
            .iter()
            .map(|pattern| rules_by_anchor.get(pattern).cloned().unwrap_or_default())
            .collect::<Vec<_>>();
        let automaton = AhoCorasickBuilder::new()
            .ascii_case_insensitive(false)
            .build(&patterns)
            .map_err(|source| {
                ModelRouteCompileError::new("route_literal_index", patterns.join("|"), source)
            })?;

        Ok(Self {
            automaton: Some(automaton),
            rules_by_pattern,
            unindexed_rule_indexes,
        })
    }

    fn candidate_rule_indexes(&self, value: &str) -> Vec<usize> {
        let mut indexes = BTreeSet::from_iter(self.unindexed_rule_indexes.iter().copied());
        if let Some(automaton) = &self.automaton {
            indexes.extend(automaton.find_iter(value).flat_map(|matched| {
                self.rules_by_pattern[matched.pattern().as_usize()]
                    .iter()
                    .copied()
            }));
        }
        indexes.into_iter().collect()
    }
}

fn route_literal_anchor_index(
    rules: &[CompiledModelRouteRule],
) -> (BTreeMap<String, Vec<usize>>, Vec<usize>) {
    let mut rules_by_anchor = BTreeMap::<String, Vec<usize>>::new();
    let mut unindexed_rule_indexes = Vec::new();
    rules.iter().enumerate().for_each(|(rule_index, rule)| {
        register_rule_literal_anchors(
            rule_index,
            rule.literal_anchors(),
            &mut rules_by_anchor,
            &mut unindexed_rule_indexes,
        );
    });
    (rules_by_anchor, unindexed_rule_indexes)
}

fn register_rule_literal_anchors(
    rule_index: usize,
    anchors: Vec<String>,
    rules_by_anchor: &mut BTreeMap<String, Vec<usize>>,
    unindexed_rule_indexes: &mut Vec<usize>,
) {
    if anchors.is_empty() {
        unindexed_rule_indexes.push(rule_index);
        return;
    }
    anchors
        .into_iter()
        .for_each(|anchor| rules_by_anchor.entry(anchor).or_default().push(rule_index));
}

fn route_literal_haystack(request: &ModelRouteRequest) -> String {
    let command_line = request.command_line();
    [
        request.executable.as_deref(),
        Some(command_line.as_str()),
        request.cwd.as_deref(),
        request.workspace.as_deref(),
        request.sub_agent_role.as_deref(),
        request.command_kind.as_ref().map(|kind| kind.as_str()),
        request.agent_scope.as_ref().map(|scope| scope.as_str()),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join("\n")
}

fn glob_literal_anchor(pattern: &str) -> Option<String> {
    let mut longest = String::new();
    let mut current = String::new();
    let mut chars = pattern.chars();
    let mut in_class = false;
    let mut brace_depth = 0usize;

    while let Some(character) = chars.next() {
        if in_class {
            if character == ']' {
                in_class = false;
            }
            continue;
        }
        if brace_depth > 0 {
            match character {
                '{' => brace_depth += 1,
                '}' => brace_depth -= 1,
                _ => {}
            }
            continue;
        }

        match character {
            '\\' => {
                if let Some(escaped) = chars.next() {
                    current.push(escaped);
                }
            }
            '*' | '?' => finish_literal_segment(&mut current, &mut longest),
            '[' => {
                finish_literal_segment(&mut current, &mut longest);
                in_class = true;
            }
            '{' => {
                finish_literal_segment(&mut current, &mut longest);
                brace_depth = 1;
            }
            _ => current.push(character),
        }
    }

    finish_literal_segment(&mut current, &mut longest);
    (!longest.is_empty()).then_some(longest)
}

fn finish_literal_segment(current: &mut String, longest: &mut String) {
    if current.len() > longest.len() {
        *longest = std::mem::take(current);
    } else {
        current.clear();
    }
}

/// Error raised when a route glob cannot be compiled.
#[derive(Debug)]
pub struct ModelRouteCompileError {
    dimension: &'static str,
    pattern: String,
    source: Box<dyn Error + Send + Sync + 'static>,
}

impl ModelRouteCompileError {
    fn new(
        dimension: &'static str,
        pattern: String,
        source: impl Error + Send + Sync + 'static,
    ) -> Self {
        Self {
            dimension,
            pattern,
            source: Box::new(source),
        }
    }

    pub fn dimension(&self) -> &'static str {
        self.dimension
    }

    pub fn pattern(&self) -> &str {
        self.pattern.as_str()
    }
}

impl Display for ModelRouteCompileError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "failed to compile {} route glob `{}`: {}",
            self.dimension, self.pattern, self.source
        )
    }
}

impl Error for ModelRouteCompileError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.source.as_ref())
    }
}
