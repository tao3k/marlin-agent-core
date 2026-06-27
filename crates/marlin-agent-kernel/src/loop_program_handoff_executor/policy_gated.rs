//! Policy-gated Agent-Flow executor for admitted runtime handoffs.

use crate::{LoopProgramRuntimeHandoff, LoopProgramRuntimeHandoffPlan};

use super::runtime::{
    AgentFlowLoopProgramRuntimeHandoffExecutor, LoopProgramRuntimeHandoffExecution,
    LoopProgramRuntimeHandoffExecutionReceipt, LoopProgramRuntimeHandoffExecutionStatus,
    LoopProgramRuntimeHandoffExecutor, LoopProgramRuntimeHandoffRouter,
};

/// Runtime executor that gates every handoff through a Rust-owned policy router
/// before projecting admitted Agent-Flow intents.
#[derive(Clone)]
pub struct PolicyGatedAgentFlowLoopProgramRuntimeHandoffExecutor {
    router: LoopProgramRuntimeHandoffRouter,
    agent_flow: AgentFlowLoopProgramRuntimeHandoffExecutor,
}

impl PolicyGatedAgentFlowLoopProgramRuntimeHandoffExecutor {
    pub fn new(
        router: LoopProgramRuntimeHandoffRouter,
        agent_flow: AgentFlowLoopProgramRuntimeHandoffExecutor,
    ) -> Self {
        Self { router, agent_flow }
    }
}

impl LoopProgramRuntimeHandoffExecutor for PolicyGatedAgentFlowLoopProgramRuntimeHandoffExecutor {
    fn execute_plan(
        &self,
        plan: &LoopProgramRuntimeHandoffPlan,
    ) -> LoopProgramRuntimeHandoffExecutionReceipt {
        let admission_receipt = self.router.execute_plan(plan);
        let admitted_agent_flow_handoffs = plan
            .handoffs
            .iter()
            .filter(|handoff| {
                handoff.agent_flow_intent.is_some()
                    && execution_for_handoff(&admission_receipt.executions, handoff).status
                        == LoopProgramRuntimeHandoffExecutionStatus::Handled
            })
            .cloned()
            .collect::<Vec<_>>();

        if admitted_agent_flow_handoffs.is_empty() {
            return admission_receipt;
        }

        let projected_plan = LoopProgramRuntimeHandoffPlan {
            program_id: plan.program_id.clone(),
            handoffs: admitted_agent_flow_handoffs.into_boxed_slice(),
        };
        let projection_receipt = self.agent_flow.execute_plan(&projected_plan);
        let executions = plan
            .handoffs
            .iter()
            .map(|handoff| {
                let admission = execution_for_handoff(&admission_receipt.executions, handoff);
                if handoff.agent_flow_intent.is_none()
                    || admission.status != LoopProgramRuntimeHandoffExecutionStatus::Handled
                {
                    return admission.clone();
                }

                let projection = execution_for_handoff(&projection_receipt.executions, handoff);
                if projection.status != LoopProgramRuntimeHandoffExecutionStatus::Handled {
                    return projection.clone();
                }

                merge_admission_and_projection(admission, projection)
            })
            .collect::<Vec<_>>();

        let LoopProgramRuntimeHandoffExecutionReceipt {
            agent_flow_receipt,
            tool_process_projections,
            memory_projections,
            ..
        } = projection_receipt;
        let mut receipt =
            LoopProgramRuntimeHandoffExecutionReceipt::new(plan.program_id.clone(), executions);
        if let Some(agent_flow_receipt) = agent_flow_receipt {
            receipt = receipt.with_agent_flow_receipt(agent_flow_receipt);
        }
        receipt
            .with_tool_process_projections(tool_process_projections)
            .with_memory_projections(memory_projections)
    }
}

fn merge_admission_and_projection(
    admission: &LoopProgramRuntimeHandoffExecution,
    projection: &LoopProgramRuntimeHandoffExecution,
) -> LoopProgramRuntimeHandoffExecution {
    let mut merged = admission.clone();
    merged.next_event.clone_from(&projection.next_event);
    merged
}

fn execution_for_handoff<'a>(
    executions: &'a [LoopProgramRuntimeHandoffExecution],
    handoff: &LoopProgramRuntimeHandoff,
) -> &'a LoopProgramRuntimeHandoffExecution {
    executions
        .iter()
        .find(|execution| {
            execution.step_index == handoff.receipt.step_index && execution.kind == handoff.kind
        })
        .expect("policy-gated runtime execution should preserve every handoff")
}
