//! Policy-combination experiment projection for intent-case artifact receipts.

use marlin_agent_kernel::{LoopProgramExecutionReceipt, LoopProgramExecutionReplayBundleReceipt};
use marlin_gerbil_scheme::{
    GerbilLoopCaseDriverCapability, GerbilLoopCaseDriverVerticalTraceReceipt,
};

pub(crate) fn render_policy_combination_experiment_lines(
    vertical_trace: &GerbilLoopCaseDriverVerticalTraceReceipt,
    execution_receipt: &LoopProgramExecutionReceipt,
    side_effect_replay_bundle: Option<&LoopProgramExecutionReplayBundleReceipt>,
) -> Vec<String> {
    let policy_combination_capability = GerbilLoopCaseDriverCapability::new("+policy-combination");
    if !vertical_trace.has_capability(&policy_combination_capability) {
        return Vec::new();
    }

    let memory_projection_count = execution_receipt
        .steps
        .iter()
        .map(|step| step.runtime_handoff_execution.memory_projections.len())
        .sum::<usize>();
    let tool_projection_count = execution_receipt
        .steps
        .iter()
        .map(|step| {
            step.runtime_handoff_execution
                .tool_process_projections
                .len()
        })
        .sum::<usize>();
    let model_step_count = action_count(execution_receipt, "InvokeModel");
    let rewrite_step_count = action_count(execution_receipt, "RewriteGraph");
    let checker_step_count = action_count(execution_receipt, "Verify");
    let tool_replay_process_count = side_effect_replay_bundle
        .map(|bundle| {
            bundle
                .step_replay_bundles
                .iter()
                .map(|step_bundle| step_bundle.side_effects.tool_processes.len())
                .sum::<usize>()
        })
        .unwrap_or_default();

    let mut missing = Vec::new();
    if vertical_trace.memory_intent_count() == 0 || memory_projection_count == 0 {
        missing.push("memory");
    }
    if model_step_count == 0 {
        missing.push("model");
    }
    if rewrite_step_count == 0 {
        missing.push("rewrite");
    }
    if vertical_trace.tool_intent_count() == 0 || tool_projection_count == 0 {
        missing.push("tool");
    }
    if side_effect_replay_bundle.is_none() || tool_replay_process_count == 0 {
        missing.push("tool-replay");
    }
    if checker_step_count == 0 {
        missing.push("checker");
    }

    let mut lines = vec![
        "policy_combination_experiment=memory-rewrite-checker".to_owned(),
        "policy_combination_expected_lanes=memory,model,rewrite,tool,tool-replay,checker"
            .to_owned(),
        format!(
            "policy_combination_memory_intent_count={}",
            vertical_trace.memory_intent_count()
        ),
        format!(
            "policy_combination_tool_intent_count={}",
            vertical_trace.tool_intent_count()
        ),
        format!("policy_combination_memory_projection_count={memory_projection_count}"),
        format!("policy_combination_model_step_count={model_step_count}"),
        format!("policy_combination_rewrite_step_count={rewrite_step_count}"),
        format!("policy_combination_tool_projection_count={tool_projection_count}"),
        format!("policy_combination_tool_replay_process_count={tool_replay_process_count}"),
        format!("policy_combination_checker_step_count={checker_step_count}"),
        format!(
            "policy_combination_side_effect_replay={}",
            side_effect_replay_bundle.is_some()
        ),
    ];

    if missing.is_empty() {
        lines.push("policy_combination_experiment_status=complete".to_owned());
        lines.push("policy_combination_recommendation_count=0".to_owned());
    } else {
        lines.push(format!(
            "policy_combination_experiment_status=missing-lanes:{}",
            missing.join(",")
        ));
        lines.push(format!(
            "policy_combination_recommendation_count={}",
            missing.len()
        ));
        for (index, lane) in missing.iter().enumerate() {
            lines.push(format!(
                "policy_combination_recommendation.{}={}",
                index + 1,
                recommendation_for_missing_lane(lane)
            ));
        }
    }

    lines
}

fn action_count(execution_receipt: &LoopProgramExecutionReceipt, action_name: &str) -> usize {
    execution_receipt
        .steps
        .iter()
        .filter(|step| format!("{:?}", step.machine_receipt.action) == action_name)
        .count()
}

fn recommendation_for_missing_lane(lane: &str) -> &'static str {
    match lane {
        "memory" => "restore-memory-recall-mixin-before-rewrite-checker",
        "model" => "ensure-model-invocation-precedes-graph-rewrite",
        "rewrite" => "restore-dynamic-rewrite-policy-before-tool-dispatch",
        "tool" => "attach-tool-sandbox-side-effect-replay",
        "tool-replay" => "materialize-real-tool-side-effect-replay-before-completing-experiment",
        "checker" => "keep-verifier-checker-as-final-policy-lane",
        _ => "inspect-policy-combination-linearization",
    }
}
