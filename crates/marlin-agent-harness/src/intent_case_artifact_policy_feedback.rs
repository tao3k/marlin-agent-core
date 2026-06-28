//! Policy feedback projection for intent-case artifact receipts.

use marlin_gerbil_scheme::GerbilLoopCaseDriverVerticalTraceReceipt;

pub(crate) fn policy_merge_feedback_lines(
    vertical_trace: &GerbilLoopCaseDriverVerticalTraceReceipt,
) -> Vec<String> {
    let merge_kinds = vertical_trace.policy_merge_kinds().collect::<Vec<_>>();
    if vertical_trace.policy_conflict_merge_receipt_count() > 0 {
        return vec![
            "policy_feedback_status=requires-profile-revision".to_owned(),
            "policy_feedback_recommendation_count=2".to_owned(),
            "policy_feedback_recommendation.1=split-exclusive-resource-policy-before-runtime-handoff".to_owned(),
            "policy_feedback_recommendation.2=rerun-intent-case-after-policy-linearization-update".to_owned(),
        ];
    }
    if merge_kinds.contains(&"conflict_error") {
        return vec![
            "policy_feedback_status=stable-resource-consensus".to_owned(),
            "policy_feedback_recommendation_count=1".to_owned(),
            "policy_feedback_recommendation.1=keep-exclusive-resource-mixins-aligned".to_owned(),
        ];
    }
    vec![
        "policy_feedback_status=stable-merge-evidence".to_owned(),
        "policy_feedback_recommendation_count=1".to_owned(),
        "policy_feedback_recommendation.1=continue-scripted-intent-case-replay".to_owned(),
    ]
}
