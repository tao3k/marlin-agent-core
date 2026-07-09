//! Real LLM case receipt projection for intent-case artifacts.

use marlin_gerbil_scheme::GerbilLoopCaseDriverRealLlmCaseReceipt;

pub(crate) fn real_llm_model_event_lines(
    receipt: Option<&GerbilLoopCaseDriverRealLlmCaseReceipt>,
) -> Vec<String> {
    let Some(receipt) = receipt else {
        return Vec::new();
    };

    vec![
        "real_llm_case_model_event=present".to_owned(),
        format!("real_llm_case_id={}", receipt.case_id()),
        format!("real_llm_case_result={}", receipt.result()),
        format!("real_llm_case_mode={}", receipt.mode()),
        format!(
            "real_llm_case_tool_intent={}",
            receipt.tool_intent().unwrap_or("none")
        ),
        format!("real_llm_case_rounds_used={}", receipt.rounds_used()),
        format!(
            "real_llm_case_no_write_enforced={}",
            receipt.no_write_enforced()
        ),
        format!(
            "real_llm_case_write_intent_absent={}",
            receipt.write_intent_absent()
        ),
        format!(
            "real_llm_case_terminal_status={}",
            receipt.terminal_status()
        ),
        format!(
            "real_llm_case_process_exit_status={}",
            receipt.process_exit_status()
        ),
        format!(
            "real_llm_case_governance_receipt_present={}",
            receipt.governance_receipt_present()
        ),
        format!(
            "real_llm_case_nono_sandbox_materialized={}",
            receipt.nono_sandbox_materialized()
        ),
        format!(
            "real_llm_case_human_audit_decision={}",
            receipt.human_audit_decision()
        ),
        "real_llm_case_internal_json_boundary=false".to_owned(),
    ]
}

pub(crate) fn real_llm_run_receipt_lines(
    receipt: Option<&GerbilLoopCaseDriverRealLlmCaseReceipt>,
) -> Vec<String> {
    let Some(receipt) = receipt else {
        return Vec::new();
    };

    vec![
        "run_receipt_real_llm_case_receipt=present".to_owned(),
        format!("run_receipt_real_llm_case_id={}", receipt.case_id()),
        format!("run_receipt_real_llm_result={}", receipt.result()),
        format!("run_receipt_real_llm_mode={}", receipt.mode()),
        format!(
            "run_receipt_real_llm_tool_intent={}",
            receipt.tool_intent().unwrap_or("none")
        ),
        format!("run_receipt_real_llm_rounds_used={}", receipt.rounds_used()),
        format!(
            "run_receipt_real_llm_no_write_enforced={}",
            receipt.no_write_enforced()
        ),
        format!(
            "run_receipt_real_llm_write_intent_absent={}",
            receipt.write_intent_absent()
        ),
        format!(
            "run_receipt_real_llm_terminal_status={}",
            receipt.terminal_status()
        ),
        format!(
            "run_receipt_real_llm_iteration_count={}",
            receipt.iteration_count()
        ),
        format!(
            "run_receipt_real_llm_process_exit_status={}",
            receipt.process_exit_status()
        ),
        format!(
            "run_receipt_real_llm_failure_classification_receipt_present={}",
            receipt.failure_classification_receipt_present()
        ),
        format!(
            "run_receipt_real_llm_governance_receipt_present={}",
            receipt.governance_receipt_present()
        ),
        format!(
            "run_receipt_real_llm_nono_sandbox_materialized={}",
            receipt.nono_sandbox_materialized()
        ),
        format!(
            "run_receipt_real_llm_human_audit_decision={}",
            receipt.human_audit_decision()
        ),
        format!(
            "run_receipt_real_llm_continuation_planner={}",
            receipt.continuation_planner().unwrap_or("none")
        ),
        "run_receipt_real_llm_internal_json_boundary=false".to_owned(),
    ]
}
