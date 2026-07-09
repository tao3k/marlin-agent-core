//! Runtime repair receipt rendering for intent-case verifier artifacts.

use marlin_agent_harness_types::RuntimeRepairCaseReceipt;

pub(crate) fn render_runtime_repair_case_receipt(receipt: &RuntimeRepairCaseReceipt) -> String {
    let mut lines = vec![
        "runtime_repair_receipt=present".to_owned(),
        format!("runtime_repair_schema={}", receipt.schema_id()),
        format!("runtime_repair_case_id={}", receipt.case_id()),
        format!("runtime_repair_profile_ref={}", receipt.profile_ref()),
    ];

    match receipt {
        RuntimeRepairCaseReceipt::Live(receipt) => {
            render_live_runtime_repair_receipt(&mut lines, receipt)
        }
        RuntimeRepairCaseReceipt::NoLive(receipt) => {
            lines.push("runtime_repair_kind=no-live".to_owned());
            lines.push(format!(
                "runtime_repair_program_id={}",
                receipt.program_id.as_str()
            ));
            lines.push(format!(
                "runtime_repair_gate_status={:?}",
                receipt.gate_status
            ));
            lines.push(format!(
                "runtime_repair_denial_reason={}",
                receipt.denial_reason
            ));
            lines.push(format!(
                "runtime_repair_live_llm_allowed={}",
                receipt.live_llm_allowed
            ));
            lines.push(format!(
                "runtime_repair_action_count={}",
                receipt.action_count.get()
            ));
            lines.push(format!(
                "runtime_repair_model_handoff_status={:?}",
                receipt.model_handoff_status
            ));
            lines.push("runtime_repair_diagnosis_status=blocked-live-llm".to_owned());
            lines.push("runtime_repair_single_file_repair_ready=false".to_owned());
            lines.push("runtime_repair_recommendation_count=1".to_owned());
            lines.push(
                "runtime_repair_recommendation.1=enable-live-llm-provider-before-runtime-repair"
                    .to_owned(),
            );
        }
    }

    lines.join("\n") + "\n"
}

fn render_live_runtime_repair_receipt(
    lines: &mut Vec<String>,
    receipt: &marlin_agent_harness_types::RuntimeRepairLiveCaseReceipt,
) {
    lines.push("runtime_repair_kind=live".to_owned());
    lines.push(format!(
        "runtime_repair_program_id={}",
        receipt.program_id.as_str()
    ));
    lines.push(format!(
        "runtime_repair_model_completion_id={}",
        receipt.model_completion_id
    ));
    lines.push(format!("runtime_repair_model={}", receipt.model));
    lines.push(format!(
        "runtime_repair_elapsed_ms={}",
        receipt.elapsed_ms.get()
    ));
    lines.push(format!(
        "runtime_repair_action_count={}",
        receipt.action_count.get()
    ));
    lines.push(format!(
        "runtime_repair_tool_projection_count={}",
        receipt.tool_projection_count.get()
    ));
    lines.push(format!(
        "runtime_repair_patch_tool_success={}",
        receipt.patch_tool_success
    ));
    lines.push(format!(
        "runtime_repair_graph_rewrite_projected={}",
        receipt.graph_rewrite_projected
    ));
    lines.push(format!(
        "runtime_repair_verification_success={}",
        receipt.verification_success
    ));
    lines.push(format!(
        "runtime_repair_repaired_content_digest={}",
        receipt.repaired_content.digest
    ));
    lines.push(format!(
        "runtime_repair_repaired_content_bytes={}",
        receipt.repaired_content.byte_count.get()
    ));
    lines.extend(live_runtime_repair_diagnosis_lines(receipt));
}

fn live_runtime_repair_diagnosis_lines(
    receipt: &marlin_agent_harness_types::RuntimeRepairLiveCaseReceipt,
) -> Vec<String> {
    let single_file_ready = receipt.patch_tool_success
        && receipt.verification_success
        && receipt.tool_projection_count.get() == 1;
    let mut recommendations = Vec::new();
    if receipt.tool_projection_count.get() == 0 {
        recommendations.push("route-tool-intent-to-dispatch-runtime");
    }
    if !receipt.patch_tool_success {
        recommendations.push("repair-patch-tool-policy-or-sandbox");
    }
    if !receipt.verification_success {
        recommendations.push("rerun-verifier-after-runtime-patch");
    }

    let diagnosis_status = if single_file_ready {
        "verified-single-file-repair"
    } else if receipt.verification_success {
        "verified-repair-needs-shape-review"
    } else {
        "requires-repair-policy-review"
    };

    let mut lines = vec![
        format!("runtime_repair_diagnosis_status={diagnosis_status}"),
        format!("runtime_repair_single_file_repair_ready={single_file_ready}"),
        format!(
            "runtime_repair_recommendation_count={}",
            recommendations.len()
        ),
    ];
    lines.extend(
        recommendations
            .iter()
            .enumerate()
            .map(|(index, recommendation)| {
                format!(
                    "runtime_repair_recommendation.{}={recommendation}",
                    index + 1
                )
            }),
    );
    lines
}
