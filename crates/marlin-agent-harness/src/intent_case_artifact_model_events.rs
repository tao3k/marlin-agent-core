//! Model event artifact rendering for intent-case bundles.

use marlin_agent_harness_types::{IntentCaseArtifactManifest, RuntimeRepairCaseReceipt};
use marlin_agent_kernel::LoopProgramExecutionReceipt;

pub(crate) fn render_model_events_artifact(
    manifest: &IntentCaseArtifactManifest,
    execution_receipt: &LoopProgramExecutionReceipt,
    runtime_repair_receipt: Option<&RuntimeRepairCaseReceipt>,
) -> String {
    let mut lines = Vec::new();
    for step in &execution_receipt.steps {
        if format!("{:?}", step.machine_receipt.action) == "InvokeModel" {
            let step_index = step.machine_receipt.step_index.get();
            lines.push(format!(
                "model step={} model_invocation_id={} status={:?}",
                step_index,
                model_invocation_id_for_step(manifest, step_index).unwrap_or("none"),
                step.runtime_handoff_execution.status
            ));
        }
    }
    if let Some(runtime_repair_receipt) = runtime_repair_receipt {
        lines.extend(render_runtime_repair_model_event_summary(
            runtime_repair_receipt,
        ));
    }
    if lines.is_empty() {
        lines.push("model=none".to_owned());
    }
    lines.join("\n") + "\n"
}

fn render_runtime_repair_model_event_summary(receipt: &RuntimeRepairCaseReceipt) -> Vec<String> {
    match receipt {
        RuntimeRepairCaseReceipt::Live(receipt) => vec![
            "runtime_repair_model_event=present".to_owned(),
            "runtime_repair_model_event_kind=live".to_owned(),
            format!("runtime_repair_model_event_schema={}", receipt.schema_id),
            format!(
                "runtime_repair_model_event_completion_id={}",
                receipt.model_completion_id
            ),
            format!("runtime_repair_model_event_model={}", receipt.model),
            format!(
                "runtime_repair_model_event_elapsed_ms={}",
                receipt.elapsed_ms.get()
            ),
        ],
        RuntimeRepairCaseReceipt::NoLive(receipt) => vec![
            "runtime_repair_model_event=present".to_owned(),
            "runtime_repair_model_event_kind=no-live".to_owned(),
            format!("runtime_repair_model_event_schema={}", receipt.schema_id),
            format!(
                "runtime_repair_model_event_gate_status={:?}",
                receipt.gate_status
            ),
            format!(
                "runtime_repair_model_event_denial_reason={}",
                receipt.denial_reason
            ),
            format!(
                "runtime_repair_model_event_live_llm_allowed={}",
                receipt.live_llm_allowed
            ),
            format!(
                "runtime_repair_model_event_handoff_status={:?}",
                receipt.model_handoff_status
            ),
        ],
    }
}

fn model_invocation_id_for_step(
    manifest: &IntentCaseArtifactManifest,
    step_index: u64,
) -> Option<&str> {
    manifest
        .trace_index
        .entries
        .iter()
        .find(|entry| entry.step_index == step_index && entry.action == "invoke_model")
        .and_then(|entry| entry.model_invocation_id.as_ref())
        .map(|id| id.as_str())
}
