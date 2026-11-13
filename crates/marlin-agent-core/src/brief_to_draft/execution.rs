// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Execution of a host-admitted Brief-to-Draft generation.

use std::sync::{Arc, Mutex};

use marlin_agent_kernel::{
    GraphLoopKernel, GraphNodeExecutor, TokioGraphLoopKernel,
    compile_graph_policy_proposal_with_c_abi_readiness,
};
use marlin_agent_protocol::{
    GraphCAbiReadinessReceipt, GraphCAbiRequirement, GraphLoopExecutionBudget,
    GraphLoopExecutionResult, GraphLoopStrategy, GraphNodeExecutionReceipt, GraphNodeInvocation,
    GraphPolicyProposal, GraphPolicyProposalReceipt, LoopEdgeSpec, LoopGraph, LoopNodeSpec, RunId,
};
use marlin_agent_runtime::{
    RuntimeContext, RuntimeFuture, TokioAgentRuntime,
    plugin::{
        ActivationId, EffectId, PluginDenial, PluginHost, WorkspaceCreate, WorkspaceEffectError,
        WorkspaceFileBroker,
    },
};
use marlin_gerbil_scheme::{
    BriefNode, BriefProposal, BriefTemplate, GerbilBriefPlanner, GerbilBriefSettingsError,
};
use serde::{Deserialize, Serialize};

use super::capability;

mod replay;
pub use replay::BriefRunEvidence;

const EXPORT: &str = "marlin_brief_to_draft_propose";
const RENDER: &str = "brief-to-draft.render";
const CREATE: &str = "brief-to-draft.create";

/// Evidence from the Scheme proposal, Rust readiness gate and kernel execution.
#[derive(Debug)]
pub struct BriefRun {
    activation: ActivationId,
    render_input: BriefRenderInput,
    pub readiness: GraphCAbiReadinessReceipt,
    pub proposal: GraphPolicyProposal,
    pub admission: GraphPolicyProposalReceipt,
    pub execution: GraphLoopExecutionResult,
    pub input: (EffectId, Vec<u8>),
    pub output: Result<Option<(EffectId, Vec<u8>)>, WorkspaceEffectError>,
}

/// A refusal before execution, or a failure observing the tracked graph task.
#[derive(Debug)]
pub enum BriefRunError {
    Activation(PluginDenial),
    Workspace(WorkspaceEffectError),
    SettingsMismatch,
    InvalidBrief,
    Policy(GerbilBriefSettingsError),
    InvalidProposal,
    Rejected(Box<GraphPolicyProposalReceipt>),
    TaskFailed,
    Cancelled,
}

/// Read a brief and execute the Scheme-selected plan on the supplied runtime.
/// The settings must match the digest admitted for this activation.
pub async fn run(
    host: &PluginHost<'_>,
    activation: &ActivationId,
    broker: &WorkspaceFileBroker,
    planner: &GerbilBriefPlanner<'_>,
    settings: (bool, BriefTemplate),
    run_id: RunId,
    runtime: &TokioAgentRuntime,
) -> Result<BriefRun, BriefRunError> {
    let (audience_required, template) = settings;
    if runtime.cancellation_token().is_cancelled() {
        return Err(BriefRunError::Cancelled);
    }
    let admitted = host
        .active_workflow(activation, &super::policy_module())
        .map_err(BriefRunError::Activation)?;
    if admitted.settings_digest != super::settings_digest(audience_required, template) {
        return Err(BriefRunError::SettingsMismatch);
    }
    let read = capability("read-brief");
    let create = capability("create-draft");
    let input = broker
        .read(host, activation, &read)
        .map_err(BriefRunError::Workspace)?;
    let source = std::str::from_utf8(&input.value).map_err(|_| BriefRunError::InvalidBrief)?;
    let plan = planner
        .propose(source, audience_required, template)
        .map_err(BriefRunError::Policy)?
        .ok_or(BriefRunError::InvalidBrief)?;
    let requirement = workflow_abi_requirement();
    let abi = planner.abi_version().map_err(BriefRunError::Policy)?;
    let readiness = GraphCAbiReadinessReceipt::evaluate(
        &requirement,
        (abi == requirement.version).then_some(EXPORT),
    );
    let render_input = BriefRenderInput::from_projection(plan, audience_required, template)?;
    let graph = graph();
    let planned = serde_json::to_vec(&graph).map_err(|_| BriefRunError::InvalidProposal)?;
    let proposal = GraphPolicyProposal::new(
        GraphLoopStrategy::gerbil_c_abi("brief-to-draft", "v0"),
        graph,
        digest_ref(input.digest),
        digest_ref(hash(&planned)),
    )
    .with_c_abi_requirement(requirement);
    let compiled =
        compile_graph_policy_proposal_with_c_abi_readiness(run_id.as_str(), &proposal, &readiness);
    let request = compiled
        .request
        .ok_or_else(|| BriefRunError::Rejected(Box::new(compiled.receipt.clone())))?
        .with_budget(GraphLoopExecutionBudget::max_node_executions(2));
    let operation = broker
        .prepare_create(host, activation, &create)
        .map_err(BriefRunError::Workspace)?;
    let state = Arc::new(Mutex::new(ExecutionState {
        render_input: Some(render_input.clone()),
        draft: None,
        create: Some(operation),
        output: Ok(None),
    }));
    let kernel = TokioGraphLoopKernel::new(request.run_id.clone(), request.graph.graph_id.clone())
        .with_executor(RENDER, BriefExecutor(state.clone()))
        .with_executor(CREATE, BriefExecutor(state.clone()));
    let execution = kernel
        .spawn_execution(request, runtime)
        .join()
        .await
        .map_err(|_| BriefRunError::TaskFailed)?;
    let output = std::mem::replace(
        &mut state.lock().map_err(|_| BriefRunError::TaskFailed)?.output,
        Ok(None),
    );
    Ok(BriefRun {
        activation: activation.clone(),
        render_input,
        readiness,
        proposal,
        admission: compiled.receipt,
        execution,
        input: (input.id, input.value),
        output,
    })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BriefRenderInput {
    title: String,
    audience: String,
    audience_required: bool,
    release_note: bool,
}

impl BriefRenderInput {
    fn from_projection(
        plan: BriefProposal,
        audience_required: bool,
        template: BriefTemplate,
    ) -> Result<Self, BriefRunError> {
        let release_note = template == BriefTemplate::ReleaseNote;
        let expected = if release_note {
            BriefNode::ReleaseNote
        } else {
            BriefNode::BriefNote
        };
        let input = Self {
            title: plan.title,
            audience: plan.audience,
            audience_required,
            release_note,
        };
        if plan.nodes != [expected, BriefNode::CreateDraft] || !input.is_valid() {
            return Err(BriefRunError::InvalidProposal);
        }
        Ok(input)
    }

    fn is_valid(&self) -> bool {
        !self.title.trim().is_empty()
            && (!self.audience_required || !self.audience.trim().is_empty())
            && (!self.release_note || self.audience_required)
            && [&self.title, &self.audience]
                .iter()
                .all(|text| !text.chars().any(char::is_control))
    }

    fn settings_digest(&self) -> [u8; 32] {
        let template = if self.release_note {
            BriefTemplate::ReleaseNote
        } else {
            BriefTemplate::BriefNote
        };
        super::settings_digest(self.audience_required, template)
    }

    fn render(&self) -> Vec<u8> {
        let label = if self.release_note {
            "Release note"
        } else {
            "Brief note"
        };
        format!("* {label}: {}\nAudience: {}\n", self.title, self.audience).into_bytes()
    }
}

fn workflow_abi_requirement() -> GraphCAbiRequirement {
    GraphCAbiRequirement::new("marlin.brief-to-draft", 1).with_required_symbols([EXPORT])
}

fn graph() -> LoopGraph {
    LoopGraph {
        graph_id: "brief-to-draft".into(),
        nodes: vec![node("render", RENDER), node("create", CREATE)],
        edges: vec![LoopEdgeSpec {
            from: "render".into(),
            to: "create".into(),
            condition: None,
        }],
    }
}

fn node(id: &str, executor: &str) -> LoopNodeSpec {
    LoopNodeSpec {
        id: id.into(),
        executor: executor.into(),
        config: Default::default(),
    }
}

fn hash(bytes: &[u8]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes).into()
}

fn digest_ref(hash: [u8; 32]) -> String {
    use std::fmt::Write;
    let mut value = String::from("sha256:");
    for byte in hash {
        write!(&mut value, "{byte:02x}").expect("write to String");
    }
    value
}

struct ExecutionState {
    render_input: Option<BriefRenderInput>,
    draft: Option<Vec<u8>>,
    create: Option<WorkspaceCreate>,
    output: Result<Option<(EffectId, Vec<u8>)>, WorkspaceEffectError>,
}

struct BriefExecutor(Arc<Mutex<ExecutionState>>);

impl GraphNodeExecutor for BriefExecutor {
    fn execute_node(
        &self,
        invocation: GraphNodeInvocation,
        context: RuntimeContext,
    ) -> RuntimeFuture<GraphNodeExecutionReceipt> {
        let state = self.0.clone();
        Box::pin(async move {
            let status = match invocation.executor.as_str() {
                RENDER => render(&state),
                CREATE => tokio::task::spawn_blocking(move || {
                    if context.is_cancelled() {
                        return Err("draft create cancelled");
                    }
                    create(&state)
                })
                .await
                .unwrap_or(Err("draft create task failed")),
                _ => Err("unregistered Brief-to-Draft executor"),
            };
            match status {
                Ok(()) => {
                    GraphNodeExecutionReceipt::completed(invocation.node_id, invocation.executor)
                }
                Err(error) => GraphNodeExecutionReceipt::failed(
                    invocation.node_id,
                    invocation.executor,
                    vec![error.into()],
                ),
            }
        })
    }
}

fn render(state: &Mutex<ExecutionState>) -> Result<(), &'static str> {
    let mut state = state.lock().map_err(|_| "workflow state unavailable")?;
    let input = state
        .render_input
        .take()
        .ok_or("draft render already executed")?;
    state.draft = Some(input.render());
    Ok(())
}

fn create(state: &Mutex<ExecutionState>) -> Result<(), &'static str> {
    let mut state = state.lock().map_err(|_| "workflow state unavailable")?;
    let draft = state.draft.take().ok_or("draft render has no output")?;
    let operation = state.create.take().ok_or("draft create already executed")?;
    state.output = operation
        .execute(&draft)
        .map(|effect| Some((effect.id, draft)));
    state
        .output
        .as_ref()
        .map(|_| ())
        .map_err(|_| "draft create refused or failed")
}
