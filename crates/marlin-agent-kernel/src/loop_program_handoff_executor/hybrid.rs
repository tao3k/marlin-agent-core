//! Hybrid runtime handoff executor for mixed Agent-Flow and direct runtime lanes.

use crate::{LoopProgramRuntimeHandoff, LoopProgramRuntimeHandoffPlan};

use super::runtime::{
    AgentFlowLoopProgramRuntimeHandoffExecutor, LoopProgramRuntimeHandoffExecution,
    LoopProgramRuntimeHandoffExecutionReceipt, LoopProgramRuntimeHandoffExecutor,
    LoopProgramRuntimeHandoffRouter,
};

/// Runtime executor that routes Agent-Flow lanes through typed projection and
/// sends every other lane through an explicit handler router.
#[derive(Clone)]
pub struct HybridLoopProgramRuntimeHandoffExecutor {
    router: LoopProgramRuntimeHandoffRouter,
    agent_flow: AgentFlowLoopProgramRuntimeHandoffExecutor,
}

impl HybridLoopProgramRuntimeHandoffExecutor {
    pub fn new(
        router: LoopProgramRuntimeHandoffRouter,
        agent_flow: AgentFlowLoopProgramRuntimeHandoffExecutor,
    ) -> Self {
        Self { router, agent_flow }
    }
}

impl LoopProgramRuntimeHandoffExecutor for HybridLoopProgramRuntimeHandoffExecutor {
    fn execute_plan(
        &self,
        plan: &LoopProgramRuntimeHandoffPlan,
    ) -> LoopProgramRuntimeHandoffExecutionReceipt {
        let agent_flow_handoffs = plan
            .handoffs
            .iter()
            .filter(|handoff| handoff.agent_flow_intent.is_some())
            .cloned()
            .collect::<Vec<_>>();
        if agent_flow_handoffs.is_empty() {
            return self.router.execute_plan(plan);
        }

        let routed_handoffs = plan
            .handoffs
            .iter()
            .filter(|handoff| handoff.agent_flow_intent.is_none())
            .cloned()
            .collect::<Vec<_>>();
        if routed_handoffs.is_empty() {
            return self.agent_flow.execute_plan(plan);
        }

        let agent_flow_plan = LoopProgramRuntimeHandoffPlan {
            program_id: plan.program_id.clone(),
            handoffs: agent_flow_handoffs.into_boxed_slice(),
        };
        let routed_plan = LoopProgramRuntimeHandoffPlan {
            program_id: plan.program_id.clone(),
            handoffs: routed_handoffs.into_boxed_slice(),
        };

        let agent_flow_receipt = self.agent_flow.execute_plan(&agent_flow_plan);
        let routed_receipt = self.router.execute_plan(&routed_plan);
        let executions = plan
            .handoffs
            .iter()
            .map(|handoff| {
                let receipt = if handoff.agent_flow_intent.is_some() {
                    &agent_flow_receipt
                } else {
                    &routed_receipt
                };
                execution_for_handoff(&receipt.executions, handoff).clone()
            })
            .collect::<Vec<_>>();

        let LoopProgramRuntimeHandoffExecutionReceipt {
            agent_flow_receipt,
            tool_process_projections,
            memory_projections,
            ..
        } = agent_flow_receipt;
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

fn execution_for_handoff<'a>(
    executions: &'a [LoopProgramRuntimeHandoffExecution],
    handoff: &LoopProgramRuntimeHandoff,
) -> &'a LoopProgramRuntimeHandoffExecution {
    executions
        .iter()
        .find(|execution| {
            execution.step_index == handoff.receipt.step_index && execution.kind == handoff.kind
        })
        .expect("split runtime execution should preserve every handoff")
}
