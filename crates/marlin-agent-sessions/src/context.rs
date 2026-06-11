//! Context visibility and isolation rules for runtime sessions.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// Named context buckets that can be exposed to a provider, hook, tool, or sub-agent.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum ContextNamespace {
    System,
    User,
    Workspace,
    Memory,
    Tools,
    Hooks,
    SubAgents,
    Secrets,
}

/// Visibility granted to a session over runtime context and history.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContextVisibility {
    namespaces: BTreeSet<ContextNamespace>,
    max_history_items: Option<usize>,
}

impl ContextVisibility {
    pub fn from_namespaces(namespaces: impl IntoIterator<Item = ContextNamespace>) -> Self {
        Self {
            namespaces: namespaces.into_iter().collect(),
            max_history_items: None,
        }
    }

    pub fn default_runtime() -> Self {
        Self::from_namespaces([
            ContextNamespace::System,
            ContextNamespace::User,
            ContextNamespace::Workspace,
            ContextNamespace::Memory,
            ContextNamespace::Tools,
            ContextNamespace::Hooks,
            ContextNamespace::SubAgents,
        ])
    }

    pub fn with_max_history_items(mut self, max_history_items: Option<usize>) -> Self {
        self.max_history_items = max_history_items;
        self
    }

    pub fn namespaces(&self) -> impl Iterator<Item = &ContextNamespace> {
        self.namespaces.iter()
    }

    pub fn contains(&self, namespace: &ContextNamespace) -> bool {
        self.namespaces.contains(namespace)
    }

    pub fn max_history_items(&self) -> Option<usize> {
        self.max_history_items
    }

    fn grant_child_visibility(&self, requested: &Self) -> (Self, Vec<ContextNamespace>, bool) {
        let namespaces = requested
            .namespaces
            .intersection(&self.namespaces)
            .cloned()
            .collect::<BTreeSet<_>>();
        let denied_namespaces = requested
            .namespaces
            .difference(&self.namespaces)
            .cloned()
            .collect::<Vec<_>>();
        let max_history_items =
            narrow_history_limit(self.max_history_items, requested.max_history_items);
        let history_limit_applied = max_history_items != requested.max_history_items;

        (
            Self {
                namespaces,
                max_history_items,
            },
            denied_namespaces,
            history_limit_applied,
        )
    }
}

impl Default for ContextVisibility {
    fn default() -> Self {
        Self::default_runtime()
    }
}

/// Rule for whether child sessions may widen parent context visibility.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ContextExpansionPolicy {
    Deny,
    Allow,
}

/// Isolation policy applied when deriving child session contexts.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionIsolationPolicy {
    context_expansion: ContextExpansionPolicy,
}

impl SessionIsolationPolicy {
    pub fn strict() -> Self {
        Self {
            context_expansion: ContextExpansionPolicy::Deny,
        }
    }

    pub fn allow_context_expansion() -> Self {
        Self {
            context_expansion: ContextExpansionPolicy::Allow,
        }
    }

    pub fn context_expansion(&self) -> &ContextExpansionPolicy {
        &self.context_expansion
    }

    pub(crate) fn grant_visibility(
        &self,
        parent: &ContextVisibility,
        requested: &ContextVisibility,
    ) -> (ContextVisibility, Vec<ContextNamespace>, bool) {
        match self.context_expansion {
            ContextExpansionPolicy::Deny => parent.grant_child_visibility(requested),
            ContextExpansionPolicy::Allow => (requested.clone(), Vec::new(), false),
        }
    }
}

impl Default for SessionIsolationPolicy {
    fn default() -> Self {
        Self::strict()
    }
}

fn narrow_history_limit(parent: Option<usize>, requested: Option<usize>) -> Option<usize> {
    match (parent, requested) {
        (Some(parent), Some(requested)) => Some(parent.min(requested)),
        (Some(parent), None) => Some(parent),
        (None, Some(requested)) => Some(requested),
        (None, None) => None,
    }
}
