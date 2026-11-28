// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Single process-wide thread-affine Gerbil Scheme policy bridge for headless
//! live harness evaluations (zero dependency on `marlin-agent-tui`).
//!
//! Owns one dedicated `"marlin-harness-scheme-vm"` OS thread so all Scheme-to-Rust
//! boundaries (`ToolingCatalogValue`, `HelmIndexPolicy`, declarative POO
//! `HarnessSuite`s, Search DSL, and Bi-Temporal Causality trajectory verification)
//! share the same process-unique Gambit/Gerbil VM safely across async Tokio tasks.

use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock, RwLock, mpsc};

use gerbil_scheme::{
    CollectionKind, GerbilRuntime, IdentifierBuffer, IdentifierKind, ObjectKind,
    RootedSchemeObject, SchemeScalar,
};
use marlin_agent_runtime::ModelToolDeclaration;
use marlin_agent_search_engine::{
    HelmIndexPolicyProjection, SearchDslExecutionRequest, SearchDslSessionExecutionRequest,
    SearchDslSessionId, SearchDslSessionRunner, SearchDslStructuredResult,
    SearchPlanProjectionError, WorkspaceSearchIndex, default_marlin_scheme_dir,
    execute_package_search_dsl_session_structured, project_helm_index_policy,
};
use marlin_agent_tooling::decode_tooling;

use super::governance::{ProjectedCedarPolicySpec, ProjectedHarnessGovernance};
use super::runner::TrajectoryEvent;

/// One factual ground-truth checkpoint projected from Scheme POO `HarnessScenario.`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedCheckpoint {
    pub label: String,
    pub any_of: Vec<String>,
}

/// One declarative agent evaluation scenario projected from `scheme/agent-testing/`.
///
/// Scenarios carry ONLY the user's natural-language `message` (zero coaching
/// system prompt) so tool selection and search convergence are driven strictly
/// by tool declarations and tool receipts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedHarnessScenario {
    pub id: String,
    pub title: String,
    pub message: String,
    pub target_max_rounds: usize,
    pub max_rounds: usize,
    pub max_tool_calls: usize,
    pub require_workspace_search: bool,
    pub enable_reflection: bool,
    pub required_tiers: Vec<String>,
    pub governance: ProjectedHarnessGovernance,
    pub checkpoints: Vec<ProjectedCheckpoint>,
}

/// Declarative suite projected from Scheme `SearchEvalSuiteValue` (or composed POO suites).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectedHarnessSuite {
    pub suite_id: String,
    pub scenarios: Vec<ProjectedHarnessScenario>,
}

/// Cryptographic Bi-Temporal Causality verdict produced by Scheme
/// `:marlin-agent-core/policy/governance#evaluate-temporal-causal-governance`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemporalCausalityVerdict {
    pub admitted: bool,
    pub temporal_status: String,
    pub replay_status: String,
    pub causal_event_count: usize,
    pub forbidden_hits: Vec<String>,
    pub proof_digest: String,
}

enum SchemeVmCommand {
    ProjectTooling {
        scheme_dir: String,
        reply: mpsc::SyncSender<Result<Vec<ModelToolDeclaration>, String>>,
    },
    ProjectHelmIndexPolicy {
        scheme_dir: String,
        reply: mpsc::SyncSender<Result<HelmIndexPolicyProjection, String>>,
    },
    ProjectHarnessSuite {
        scheme_dir: String,
        suite_selector: Option<String>,
        reply: mpsc::SyncSender<Result<ProjectedHarnessSuite, String>>,
    },
    AnalyzeTrajectory {
        scheme_dir: String,
        scenario_id: String,
        write_allowed: bool,
        events: Vec<TrajectoryEvent>,
        reply: mpsc::SyncSender<Result<TemporalCausalityVerdict, String>>,
    },
    ExecuteSearchDslSession {
        scheme_dir: String,
        index: Arc<RwLock<WorkspaceSearchIndex>>,
        query: String,
        output_limit: usize,
        session_id: SearchDslSessionId,
        reply: mpsc::SyncSender<Result<SearchDslStructuredResult, SearchPlanProjectionError>>,
    },
}

#[derive(Default)]
struct CachedVmExports<'a> {
    tooling: Option<(String, RootedSchemeObject<'a>)>,
    harness_suite: Option<(String, RootedSchemeObject<'a>)>,
    harness_trajectory: Option<(String, RootedSchemeObject<'a>)>,
}

fn harness_scheme_vm_sender() -> &'static mpsc::Sender<SchemeVmCommand> {
    static WORKER: OnceLock<mpsc::Sender<SchemeVmCommand>> = OnceLock::new();
    WORKER.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<SchemeVmCommand>();
        std::thread::Builder::new()
            .name("marlin-harness-scheme-vm".to_owned())
            .spawn(move || {
                let runtime = GerbilRuntime::initialize();
                let mut exports = CachedVmExports::default();
                while let Ok(cmd) = rx.recv() {
                    match &runtime {
                        Err(err) => {
                            let msg = format!("Gerbil Scheme VM init failed: {err}");
                            match cmd {
                                SchemeVmCommand::ProjectTooling { reply, .. } => {
                                    let _ = reply.send(Err(msg));
                                }
                                SchemeVmCommand::ProjectHelmIndexPolicy { reply, .. } => {
                                    let _ = reply.send(Err(msg));
                                }
                                SchemeVmCommand::ProjectHarnessSuite { reply, .. } => {
                                    let _ = reply.send(Err(msg));
                                }
                                SchemeVmCommand::AnalyzeTrajectory { reply, .. } => {
                                    let _ = reply.send(Err(msg));
                                }
                                SchemeVmCommand::ExecuteSearchDslSession { reply, .. } => {
                                    let _ = reply.send(Err(SearchPlanProjectionError::Native(msg)));
                                }
                            }
                        }
                        Ok(rt) => handle_vm_command(rt, cmd, &mut exports),
                    }
                }
            })
            .expect("spawn marlin-harness-scheme-vm thread");
        tx
    })
}

fn handle_vm_command<'a>(
    rt: &'a GerbilRuntime,
    cmd: SchemeVmCommand,
    exports: &mut CachedVmExports<'a>,
) {
    match cmd {
        SchemeVmCommand::ProjectTooling { scheme_dir, reply } => {
            let outcome = project_tooling_on_vm(rt, &scheme_dir, &mut exports.tooling);
            let _ = reply.send(outcome);
        }
        SchemeVmCommand::ProjectHelmIndexPolicy { scheme_dir, reply } => {
            let outcome = project_helm_index_policy(rt, &scheme_dir).map_err(|e| e.to_string());
            let _ = reply.send(outcome);
        }
        SchemeVmCommand::ProjectHarnessSuite {
            scheme_dir,
            suite_selector,
            reply,
        } => {
            let outcome = project_harness_suite_on_vm(
                rt,
                &scheme_dir,
                suite_selector.as_deref(),
                &mut exports.harness_suite,
            );
            let _ = reply.send(outcome);
        }
        SchemeVmCommand::AnalyzeTrajectory {
            scheme_dir,
            scenario_id,
            write_allowed,
            events,
            reply,
        } => {
            let outcome = analyze_trajectory_on_vm(
                rt,
                &scheme_dir,
                &scenario_id,
                write_allowed,
                &events,
                &mut exports.harness_trajectory,
            );
            let _ = reply.send(outcome);
        }
        SchemeVmCommand::ExecuteSearchDslSession {
            scheme_dir,
            index,
            query,
            output_limit,
            session_id,
            reply,
        } => {
            let outcome = match index.read() {
                Ok(guard) => {
                    let req = SearchDslSessionExecutionRequest {
                        execution: SearchDslExecutionRequest {
                            index: &guard,
                            query: &query,
                            output_limit,
                        },
                        session_id: &session_id,
                    };
                    execute_package_search_dsl_session_structured(rt, &scheme_dir, &req)
                }
                Err(_) => Err(SearchPlanProjectionError::Invalid(
                    "resident search index lock poisoned",
                )),
            };
            let _ = reply.send(outcome);
        }
    }
}

fn resolve_cached_export<'a, 'b>(
    rt: &'a GerbilRuntime,
    scheme_dir: &str,
    module_name: &str,
    export_name: &str,
    slot: &'b mut Option<(String, RootedSchemeObject<'a>)>,
) -> Result<&'b RootedSchemeObject<'a>, String> {
    let needs_resolve = slot.as_ref().is_none_or(|(dir, _)| dir != scheme_dir);
    if needs_resolve {
        let module = rt
            .intern_identifier(IdentifierKind::Symbol, module_name)
            .map_err(|e| format!("intern `{module_name}` failed: {e}"))?;
        let sym = rt
            .intern_identifier(IdentifierKind::Symbol, export_name)
            .map_err(|e| format!("intern `{export_name}` failed: {e}"))?;
        let proc = rt
            .resolve_package_export(scheme_dir, &module, &sym)
            .map_err(|e| format!("resolve `{module_name}#{export_name}` failed: {e}"))?;
        *slot = Some((scheme_dir.to_owned(), proc));
    }
    Ok(&slot.as_ref().expect("resolved export").1)
}

fn project_tooling_on_vm<'a>(
    rt: &'a GerbilRuntime,
    scheme_dir: &str,
    slot: &mut Option<(String, RootedSchemeObject<'a>)>,
) -> Result<Vec<ModelToolDeclaration>, String> {
    let proc = resolve_cached_export(
        rt,
        scheme_dir,
        ":marlin-agent-core/c-abi/tooling",
        "marlin-tooling-project",
        slot,
    )?;
    let void_input = rt
        .void_object()
        .map_err(|e| format!("void object failed: {e}"))?;
    let projected = rt
        .call_rooted(proc, &void_input)
        .map_err(|e| format!("call marlin-tooling-project failed: {e}"))?;
    decode_tooling(projected).map_err(|e| e.to_string())
}

fn project_harness_suite_on_vm<'a>(
    rt: &'a GerbilRuntime,
    scheme_dir: &str,
    suite_selector: Option<&str>,
    slot: &mut Option<(String, RootedSchemeObject<'a>)>,
) -> Result<ProjectedHarnessSuite, String> {
    let proc = resolve_cached_export(
        rt,
        scheme_dir,
        ":marlin-agent-core/c-abi/agent-testing",
        "marlin-agent-testing-project-suite",
        slot,
    )?;
    let selector_input = match suite_selector {
        Some(name) if !name.trim().is_empty() => rt
            .intern_identifier(IdentifierKind::Symbol, name.trim())
            .map_err(|e| format!("intern suite selector `{name}` failed: {e}"))?,
        _ => rt
            .void_object()
            .map_err(|e| format!("void object failed: {e}"))?,
    };
    let projected = rt
        .call_rooted(proc, &selector_input)
        .map_err(|e| format!("call marlin-agent-testing-project-suite failed: {e}"))?;
    decode_harness_suite(projected)
}

fn analyze_trajectory_on_vm<'a>(
    rt: &'a GerbilRuntime,
    scheme_dir: &str,
    scenario_id: &str,
    write_allowed: bool,
    events: &[TrajectoryEvent],
    slot: &mut Option<(String, RootedSchemeObject<'a>)>,
) -> Result<TemporalCausalityVerdict, String> {
    let proc = resolve_cached_export(
        rt,
        scheme_dir,
        ":marlin-agent-core/c-abi/agent-testing",
        "marlin-agent-testing-verify-trajectory",
        slot,
    )?;

    let tag = rt
        .intern_identifier(IdentifierKind::Symbol, "trajectory-request")
        .map_err(|e| e.to_string())?;
    let scenario_obj = rt
        .string_from_utf8(scenario_id)
        .map_err(|e| e.to_string())?
        .into_object();
    let write_obj = rt
        .scalar_object(SchemeScalar::Boolean(write_allowed))
        .map_err(|e| e.to_string())?;

    let mut encoded_events = Vec::with_capacity(events.len());
    for ev in events {
        let ev_id = rt
            .string_from_utf8(&ev.event_id)
            .map_err(|e| e.to_string())?
            .into_object();
        let rt_sym = rt
            .intern_identifier(IdentifierKind::Symbol, ev.runtime_symbol())
            .map_err(|e| e.to_string())?;
        let pos = rt
            .scalar_object(SchemeScalar::Fixnum(
                isize::try_from(ev.logical_pos).unwrap_or(isize::MAX),
            ))
            .map_err(|e| e.to_string())?;
        let mut parent_objs = Vec::with_capacity(ev.parent_ids.len());
        for parent in &ev.parent_ids {
            parent_objs.push(
                rt.string_from_utf8(parent)
                    .map_err(|e| e.to_string())?
                    .into_object(),
            );
        }
        let parent_refs: Vec<&RootedSchemeObject<'_>> = parent_objs.iter().collect();
        let parents_vec = rt
            .collection_from_objects(CollectionKind::Vector, &parent_refs)
            .map_err(|e| e.to_string())?
            .into_object();
        let valid_pos = rt
            .scalar_object(SchemeScalar::Fixnum(
                isize::try_from(ev.valid_pos).unwrap_or(isize::MAX),
            ))
            .map_err(|e| e.to_string())?;
        let know_pos = rt
            .scalar_object(SchemeScalar::Fixnum(
                isize::try_from(ev.knowledge_pos).unwrap_or(isize::MAX),
            ))
            .map_err(|e| e.to_string())?;
        let row = rt
            .collection_from_objects(
                CollectionKind::Vector,
                &[&ev_id, &rt_sym, &pos, &parents_vec, &valid_pos, &know_pos],
            )
            .map_err(|e| e.to_string())?
            .into_object();
        encoded_events.push(row);
    }

    let event_refs: Vec<&RootedSchemeObject<'_>> = encoded_events.iter().collect();
    let events_vec = rt
        .collection_from_objects(CollectionKind::Vector, &event_refs)
        .map_err(|e| e.to_string())?
        .into_object();
    let req_vec = rt
        .collection_from_objects(
            CollectionKind::Vector,
            &[&tag, &scenario_obj, &write_obj, &events_vec],
        )
        .map_err(|e| e.to_string())?
        .into_object();

    let raw = rt
        .call_rooted(proc, &req_vec)
        .map_err(|e| format!("call marlin-agent-harness-analyze-trajectory failed: {e}"))?;
    decode_trajectory_verdict(raw)
}

fn decode_sym(val: RootedSchemeObject<'_>) -> Result<String, String> {
    if val.as_ref().kind().map_err(|e| e.to_string())? != ObjectKind::Symbol {
        return Err("expected Scheme symbol".to_owned());
    }
    let mut buf = IdentifierBuffer::default();
    val.as_ref()
        .copy_identifier_names(&mut buf)
        .map_err(|e| e.to_string())?;
    let Some((IdentifierKind::Symbol, name)) = buf.get(0) else {
        return Err("invalid Scheme symbol".to_owned());
    };
    Ok(name.to_owned())
}

fn decode_str(val: RootedSchemeObject<'_>) -> Result<String, String> {
    val.into_string()
        .map_err(|e| e.to_string())?
        .to_string()
        .into_result()
        .map_err(|e| e.to_string())
}

fn decode_bool(val: RootedSchemeObject<'_>) -> Result<bool, String> {
    match val.as_ref().scalar().map_err(|e| e.to_string())? {
        SchemeScalar::Boolean(b) => Ok(b),
        _ => Err("expected Scheme boolean".to_owned()),
    }
}

fn decode_usize(val: RootedSchemeObject<'_>) -> Result<usize, String> {
    match val.as_ref().scalar().map_err(|e| e.to_string())? {
        SchemeScalar::Fixnum(n) if n >= 0 => {
            usize::try_from(n).map_err(|_| "fixnum overflow".to_owned())
        }
        _ => Err("expected non-negative fixnum".to_owned()),
    }
}

fn decode_sym_vec(val: RootedSchemeObject<'_>) -> Result<Vec<String>, String> {
    let vec = val
        .into_collection(CollectionKind::Vector)
        .map_err(|e| e.to_string())?;
    let mut out = Vec::with_capacity(vec.len());
    for i in 0..vec.len() {
        out.push(decode_sym(vec.element(i).map_err(|e| e.to_string())?)?);
    }
    Ok(out)
}

fn decode_str_vec(val: RootedSchemeObject<'_>) -> Result<Vec<String>, String> {
    let vec = val
        .into_collection(CollectionKind::Vector)
        .map_err(|e| e.to_string())?;
    let mut out = Vec::with_capacity(vec.len());
    for i in 0..vec.len() {
        out.push(decode_str(vec.element(i).map_err(|e| e.to_string())?)?);
    }
    Ok(out)
}

fn decode_harness_suite(val: RootedSchemeObject<'_>) -> Result<ProjectedHarnessSuite, String> {
    let root = val
        .into_collection(CollectionKind::Vector)
        .map_err(|e| e.to_string())?;
    if root.len() == 2 && decode_sym(root.element(0).map_err(|e| e.to_string())?)? == "testing-error"
    {
        return Err(decode_str(root.element(1).map_err(|e| e.to_string())?)?);
    }
    if root.len() != 3 || decode_sym(root.element(0).map_err(|e| e.to_string())?)? != "testing-suite"
    {
        return Err("expected #(testing-suite id scenarios)".to_owned());
    }
    let suite_id = decode_sym(root.element(1).map_err(|e| e.to_string())?)?;
    let raw_scenarios = root
        .element(2)
        .map_err(|e| e.to_string())?
        .into_collection(CollectionKind::Vector)
        .map_err(|e| e.to_string())?;

    let mut scenarios = Vec::with_capacity(raw_scenarios.len());
    for i in 0..raw_scenarios.len() {
        let s = raw_scenarios
            .element(i)
            .map_err(|e| e.to_string())?
            .into_collection(CollectionKind::Vector)
            .map_err(|e| e.to_string())?;
        if s.len() != 11 {
            return Err(format!("expected 11-slot scenario vector, got {}", s.len()));
        }
        let gov_raw = s
            .element(7)
            .map_err(|e| e.to_string())?
            .into_collection(CollectionKind::Vector)
            .map_err(|e| e.to_string())?;
        if gov_raw.len() != 8 {
            return Err(format!(
                "expected 8-slot governance vector, got {}",
                gov_raw.len()
            ));
        }
        let cedar_raw = gov_raw
            .element(6)
            .map_err(|e| e.to_string())?
            .into_collection(CollectionKind::Vector)
            .map_err(|e| e.to_string())?;
        let mut cedar_policies = Vec::with_capacity(cedar_raw.len());
        for c_idx in 0..cedar_raw.len() {
            let entry = cedar_raw
                .element(c_idx)
                .map_err(|e| e.to_string())?
                .into_collection(CollectionKind::Vector)
                .map_err(|e| e.to_string())?;
            if entry.len() != 2 {
                return Err("expected 2-slot Cedar policy entry".to_owned());
            }
            cedar_policies.push(ProjectedCedarPolicySpec {
                policy_id: decode_sym(entry.element(0).map_err(|e| e.to_string())?)?,
                source: decode_str(entry.element(1).map_err(|e| e.to_string())?)?,
            });
        }
        let governance = ProjectedHarnessGovernance {
            policy_id: decode_sym(gov_raw.element(0).map_err(|e| e.to_string())?)?,
            write_allowed: decode_bool(gov_raw.element(1).map_err(|e| e.to_string())?)?,
            allowed_capabilities: decode_sym_vec(gov_raw.element(2).map_err(|e| e.to_string())?)?,
            denied_capabilities: decode_sym_vec(gov_raw.element(3).map_err(|e| e.to_string())?)?,
            allowed_tools: decode_sym_vec(gov_raw.element(4).map_err(|e| e.to_string())?)?,
            denied_tools: decode_sym_vec(gov_raw.element(5).map_err(|e| e.to_string())?)?,
            cedar_policies,
            denial_message: decode_str(gov_raw.element(7).map_err(|e| e.to_string())?)?,
        };

        let cps_raw = s
            .element(8)
            .map_err(|e| e.to_string())?
            .into_collection(CollectionKind::Vector)
            .map_err(|e| e.to_string())?;
        let mut checkpoints = Vec::with_capacity(cps_raw.len());
        for cp_idx in 0..cps_raw.len() {
            let entry = cps_raw
                .element(cp_idx)
                .map_err(|e| e.to_string())?
                .into_collection(CollectionKind::Vector)
                .map_err(|e| e.to_string())?;
            if entry.len() != 2 {
                return Err("expected 2-slot checkpoint entry".to_owned());
            }
            checkpoints.push(ProjectedCheckpoint {
                label: decode_str(entry.element(0).map_err(|e| e.to_string())?)?,
                any_of: decode_str_vec(entry.element(1).map_err(|e| e.to_string())?)?,
            });
        }

        scenarios.push(ProjectedHarnessScenario {
            id: decode_sym(s.element(0).map_err(|e| e.to_string())?)?,
            title: decode_str(s.element(1).map_err(|e| e.to_string())?)?,
            message: decode_str(s.element(2).map_err(|e| e.to_string())?)?,
            target_max_rounds: decode_usize(s.element(3).map_err(|e| e.to_string())?)?,
            max_rounds: decode_usize(s.element(4).map_err(|e| e.to_string())?)?,
            max_tool_calls: decode_usize(s.element(5).map_err(|e| e.to_string())?)?,
            require_workspace_search: decode_bool(s.element(6).map_err(|e| e.to_string())?)?,
            enable_reflection: decode_bool(s.element(9).map_err(|e| e.to_string())?)?,
            required_tiers: decode_sym_vec(s.element(10).map_err(|e| e.to_string())?)?,
            governance,
            checkpoints,
        });
    }

    Ok(ProjectedHarnessSuite {
        suite_id,
        scenarios,
    })
}

fn decode_trajectory_verdict(val: RootedSchemeObject<'_>) -> Result<TemporalCausalityVerdict, String> {
    let vec = val
        .into_collection(CollectionKind::Vector)
        .map_err(|e| e.to_string())?;
    if vec.len() == 2 && decode_sym(vec.element(0).map_err(|e| e.to_string())?)? == "testing-error" {
        return Err(decode_str(vec.element(1).map_err(|e| e.to_string())?)?);
    }
    if vec.len() != 7
        || decode_sym(vec.element(0).map_err(|e| e.to_string())?)? != "trajectory-verdict"
    {
        return Err("expected 7-slot #(trajectory-verdict ...)".to_owned());
    }
    Ok(TemporalCausalityVerdict {
        admitted: decode_bool(vec.element(1).map_err(|e| e.to_string())?)?,
        temporal_status: decode_sym(vec.element(2).map_err(|e| e.to_string())?)?,
        replay_status: decode_sym(vec.element(3).map_err(|e| e.to_string())?)?,
        causal_event_count: decode_usize(vec.element(4).map_err(|e| e.to_string())?)?,
        forbidden_hits: decode_sym_vec(vec.element(5).map_err(|e| e.to_string())?)?,
        proof_digest: decode_str(vec.element(6).map_err(|e| e.to_string())?)?,
    })
}

/// Thread-safe handle to Marlin's compiled Gerbil Scheme policy & harness layer.
#[derive(Clone, Debug)]
pub struct HarnessSchemeBridge {
    scheme_dir: String,
}

impl HarnessSchemeBridge {
    /// Create a bridge pointing at a compiled Marlin `scheme/` directory.
    pub fn new(scheme_dir: impl AsRef<Path>) -> Result<Self, String> {
        let dir = scheme_dir.as_ref();
        if !dir.join("gerbil.pkg").is_file() {
            return Err(format!(
                "invalid Marlin scheme directory `{}` (missing gerbil.pkg)",
                dir.display()
            ));
        }
        let utf8 = dir
            .to_str()
            .ok_or_else(|| format!("non-UTF-8 scheme directory `{}`", dir.display()))?
            .to_owned();
        Ok(Self { scheme_dir: utf8 })
    }

    /// Resolve the default repository `scheme/` directory.
    pub fn from_workspace(repo_root: &Path) -> Result<Self, String> {
        let candidate: PathBuf = repo_root.join("scheme");
        if candidate.join("gerbil.pkg").is_file() {
            return Self::new(candidate);
        }
        let fallback = default_marlin_scheme_dir().ok_or_else(|| {
            "compiled Marlin scheme/ directory not found (run `just scheme-build` or set MARLIN_SCHEME_DIR)".to_owned()
        })?;
        Self::new(fallback)
    }

    /// Project the live Scheme `ToolingCatalogValue` (`bash`, `workspace_search`, ...).
    pub fn project_tooling_declarations(&self) -> Result<Vec<ModelToolDeclaration>, String> {
        let (tx, rx) = mpsc::sync_channel(1);
        harness_scheme_vm_sender()
            .send(SchemeVmCommand::ProjectTooling {
                scheme_dir: self.scheme_dir.clone(),
                reply: tx,
            })
            .map_err(|_| "harness Scheme VM worker stopped".to_owned())?;
        rx.recv()
            .map_err(|_| "harness Scheme VM reply dropped".to_owned())?
    }

    /// Project Scheme-owned `HelmIndexPolicy.`.
    pub fn project_helm_index_policy(&self) -> Result<HelmIndexPolicyProjection, String> {
        let (tx, rx) = mpsc::sync_channel(1);
        harness_scheme_vm_sender()
            .send(SchemeVmCommand::ProjectHelmIndexPolicy {
                scheme_dir: self.scheme_dir.clone(),
                reply: tx,
            })
            .map_err(|_| "harness Scheme VM worker stopped".to_owned())?;
        rx.recv()
            .map_err(|_| "harness Scheme VM reply dropped".to_owned())?
    }

    /// Project the default declarative POO `SearchTestingSuiteValue` from Scheme.
    pub fn project_harness_suite(&self) -> Result<ProjectedHarnessSuite, String> {
        self.project_harness_suite_named(None)
    }

    /// Project a named declarative POO testing suite (`None` / `"default"` or `"complex"`)
    /// from Scheme `scheme/agent-testing/`.
    pub fn project_harness_suite_named(
        &self,
        suite_selector: Option<&str>,
    ) -> Result<ProjectedHarnessSuite, String> {
        let (tx, rx) = mpsc::sync_channel(1);
        harness_scheme_vm_sender()
            .send(SchemeVmCommand::ProjectHarnessSuite {
                scheme_dir: self.scheme_dir.clone(),
                suite_selector: suite_selector.map(str::to_owned),
                reply: tx,
            })
            .map_err(|_| "harness Scheme VM worker stopped".to_owned())?;
        rx.recv()
            .map_err(|_| "harness Scheme VM reply dropped".to_owned())?
    }

    /// Verify an observed live-agent causal trajectory through Marlin's Scheme
    /// Bi-Temporal Causality & ASCENT Temporal Lens (`evaluate-temporal-causal-governance`).
    pub fn analyze_trajectory(
        &self,
        scenario_id: &str,
        write_allowed: bool,
        events: Vec<TrajectoryEvent>,
    ) -> Result<TemporalCausalityVerdict, String> {
        let (tx, rx) = mpsc::sync_channel(1);
        harness_scheme_vm_sender()
            .send(SchemeVmCommand::AnalyzeTrajectory {
                scheme_dir: self.scheme_dir.clone(),
                scenario_id: scenario_id.to_owned(),
                write_allowed,
                events,
                reply: tx,
            })
            .map_err(|_| "harness Scheme VM worker stopped".to_owned())?;
        rx.recv()
            .map_err(|_| "harness Scheme VM reply dropped".to_owned())?
    }

    /// Return a session-aware `SearchDslSessionRunner` backing the loopback
    /// `marlin.search.v1.SearchGateway` gRPC service on this single thread-affine
    /// Scheme VM worker.
    #[must_use]
    pub fn search_session_dsl_runner(&self) -> SearchDslSessionRunner {
        let scheme_dir = self.scheme_dir.clone();
        Arc::new(move |index, query, output_limit, session_id| {
            let (tx, rx) = mpsc::sync_channel(1);
            harness_scheme_vm_sender()
                .send(SchemeVmCommand::ExecuteSearchDslSession {
                    scheme_dir: scheme_dir.clone(),
                    index,
                    query: query.to_owned(),
                    output_limit,
                    session_id: session_id.clone(),
                    reply: tx,
                })
                .map_err(|_| {
                    SearchPlanProjectionError::Invalid("harness Scheme VM worker stopped")
                })?;
            rx.recv().map_err(|_| {
                SearchPlanProjectionError::Invalid("harness Scheme VM reply dropped")
            })?
        })
    }
}
