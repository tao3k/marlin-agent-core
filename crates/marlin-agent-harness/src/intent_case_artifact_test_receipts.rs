//! Test receipt artifact rendering for intent-case bundles.

use marlin_agent_harness_types::{IntentCaseArtifactManifest, RuntimeRepairCaseReceipt};
use marlin_agent_kernel::{
    LoopProgramExecutionReplayBundleReceipt, LoopProgramFileWriteSideEffectStatus,
};

pub(crate) const INTENT_CASE_TEST_ARTIFACT_RECEIPT_SCHEMA_ID: &str =
    "marlin.intent-case.test-artifact-receipt.v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum IntentCaseTestArtifactPhase {
    Before,
    After,
}

impl IntentCaseTestArtifactPhase {
    fn as_str(self) -> &'static str {
        match self {
            Self::Before => "before",
            Self::After => "after",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum IntentCaseTestArtifactMode {
    Scripted,
    SideEffectReplay,
    RuntimeRepair,
}

impl IntentCaseTestArtifactMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Scripted => "scripted",
            Self::SideEffectReplay => "side-effect-replay",
            Self::RuntimeRepair => "runtime-repair",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum IntentCaseTestArtifactStatus {
    NotRun,
    Observed,
    Blocked,
    Verified,
    Failed,
}

impl IntentCaseTestArtifactStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::NotRun => "not-run",
            Self::Observed => "observed",
            Self::Blocked => "blocked",
            Self::Verified => "verified",
            Self::Failed => "failed",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct IntentCaseTestArtifactReceipt {
    phase: IntentCaseTestArtifactPhase,
    mode: IntentCaseTestArtifactMode,
    status: IntentCaseTestArtifactStatus,
    policy_status: String,
    execution_status: String,
    tool_process_count: usize,
    file_write_count: usize,
    completed_file_write_count: usize,
    denied_file_write_count: usize,
    patch_bytes_written: u64,
    runtime_repair_kind: &'static str,
    runtime_repair_verification_success: Option<bool>,
}

impl IntentCaseTestArtifactReceipt {
    fn from_inputs(
        phase: IntentCaseTestArtifactPhase,
        side_effect_replay_bundle: Option<&LoopProgramExecutionReplayBundleReceipt>,
        runtime_repair_receipt: Option<&RuntimeRepairCaseReceipt>,
    ) -> Self {
        let mode = receipt_mode(side_effect_replay_bundle, runtime_repair_receipt);
        let status = receipt_status(phase, side_effect_replay_bundle, runtime_repair_receipt);
        let side_effect_summary = SideEffectTestSummary::from_bundle(side_effect_replay_bundle);
        let runtime_repair_summary = RuntimeRepairTestSummary::from_receipt(runtime_repair_receipt);

        Self {
            phase,
            mode,
            status,
            policy_status: side_effect_summary.policy_status,
            execution_status: side_effect_summary.execution_status,
            tool_process_count: side_effect_summary.tool_process_count,
            file_write_count: side_effect_summary.file_write_count,
            completed_file_write_count: side_effect_summary.completed_file_write_count,
            denied_file_write_count: side_effect_summary.denied_file_write_count,
            patch_bytes_written: side_effect_summary.patch_bytes_written,
            runtime_repair_kind: runtime_repair_summary.kind,
            runtime_repair_verification_success: runtime_repair_summary.verification_success,
        }
    }

    fn render(&self, manifest: &IntentCaseArtifactManifest) -> String {
        let mut lines = vec![
            format!("test_receipt_schema={INTENT_CASE_TEST_ARTIFACT_RECEIPT_SCHEMA_ID}"),
            format!("test_receipt_case_id={}", manifest.case_id),
            format!("test_receipt_run_id={}", manifest.run_id),
            format!("test_receipt_loop_program_id={}", manifest.loop_program_id),
            format!("test_receipt_phase={}", self.phase.as_str()),
            format!("test_receipt_mode={}", self.mode.as_str()),
            format!("test_receipt_status={}", self.status.as_str()),
            format!("test_receipt_policy_status={}", self.policy_status),
            format!("test_receipt_execution_status={}", self.execution_status),
            format!(
                "test_receipt_tool_process_count={}",
                self.tool_process_count
            ),
            format!("test_receipt_file_write_count={}", self.file_write_count),
            format!(
                "test_receipt_completed_file_write_count={}",
                self.completed_file_write_count
            ),
            format!(
                "test_receipt_denied_file_write_count={}",
                self.denied_file_write_count
            ),
            format!(
                "test_receipt_patch_bytes_written={}",
                self.patch_bytes_written
            ),
            format!(
                "test_receipt_runtime_repair_kind={}",
                self.runtime_repair_kind
            ),
        ];
        lines.push(format!(
            "test_receipt_runtime_repair_verification_success={}",
            self.runtime_repair_verification_success
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_owned())
        ));
        lines.join("\n") + "\n"
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SideEffectTestSummary {
    policy_status: String,
    execution_status: String,
    tool_process_count: usize,
    file_write_count: usize,
    completed_file_write_count: usize,
    denied_file_write_count: usize,
    patch_bytes_written: u64,
}

impl SideEffectTestSummary {
    fn from_bundle(
        side_effect_replay_bundle: Option<&LoopProgramExecutionReplayBundleReceipt>,
    ) -> Self {
        let Some(bundle) = side_effect_replay_bundle else {
            return Self {
                policy_status: "none".to_owned(),
                execution_status: "none".to_owned(),
                tool_process_count: 0,
                file_write_count: 0,
                completed_file_write_count: 0,
                denied_file_write_count: 0,
                patch_bytes_written: 0,
            };
        };

        let tool_process_count = bundle
            .step_replay_bundles
            .iter()
            .map(|step| step.side_effects.tool_processes.len())
            .sum::<usize>();
        let file_writes = bundle
            .step_replay_bundles
            .iter()
            .flat_map(|step| step.side_effects.file_writes.iter())
            .collect::<Vec<_>>();
        let completed_file_write_count = file_writes
            .iter()
            .filter(|file_write| {
                file_write.status == LoopProgramFileWriteSideEffectStatus::Completed
            })
            .count();
        let denied_file_write_count = file_writes
            .iter()
            .filter(|file_write| file_write.status == LoopProgramFileWriteSideEffectStatus::Denied)
            .count();
        let patch_bytes_written = file_writes
            .iter()
            .filter_map(|file_write| file_write.write_receipt.as_ref())
            .map(|write_receipt| write_receipt.bytes_written as u64)
            .sum::<u64>();

        Self {
            policy_status: format!("{:?}", bundle.policy_status),
            execution_status: format!("{:?}", bundle.execution_status),
            tool_process_count,
            file_write_count: file_writes.len(),
            completed_file_write_count,
            denied_file_write_count,
            patch_bytes_written,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct RuntimeRepairTestSummary {
    kind: &'static str,
    verification_success: Option<bool>,
}

impl RuntimeRepairTestSummary {
    fn from_receipt(runtime_repair_receipt: Option<&RuntimeRepairCaseReceipt>) -> Self {
        match runtime_repair_receipt {
            Some(RuntimeRepairCaseReceipt::Live(receipt)) => Self {
                kind: "live",
                verification_success: Some(receipt.verification_success),
            },
            Some(RuntimeRepairCaseReceipt::NoLive(_)) => Self {
                kind: "no-live",
                verification_success: None,
            },
            None => Self {
                kind: "none",
                verification_success: None,
            },
        }
    }
}

pub(crate) fn render_test_artifact(
    manifest: &IntentCaseArtifactManifest,
    side_effect_replay_bundle: Option<&LoopProgramExecutionReplayBundleReceipt>,
    runtime_repair_receipt: Option<&RuntimeRepairCaseReceipt>,
    phase: IntentCaseTestArtifactPhase,
) -> String {
    IntentCaseTestArtifactReceipt::from_inputs(
        phase,
        side_effect_replay_bundle,
        runtime_repair_receipt,
    )
    .render(manifest)
}

fn receipt_mode(
    side_effect_replay_bundle: Option<&LoopProgramExecutionReplayBundleReceipt>,
    runtime_repair_receipt: Option<&RuntimeRepairCaseReceipt>,
) -> IntentCaseTestArtifactMode {
    if side_effect_replay_bundle.is_some() {
        IntentCaseTestArtifactMode::SideEffectReplay
    } else if runtime_repair_receipt.is_some() {
        IntentCaseTestArtifactMode::RuntimeRepair
    } else {
        IntentCaseTestArtifactMode::Scripted
    }
}

fn receipt_status(
    phase: IntentCaseTestArtifactPhase,
    side_effect_replay_bundle: Option<&LoopProgramExecutionReplayBundleReceipt>,
    runtime_repair_receipt: Option<&RuntimeRepairCaseReceipt>,
) -> IntentCaseTestArtifactStatus {
    if let Some(bundle) = side_effect_replay_bundle {
        if bundle.blocks_replay() {
            IntentCaseTestArtifactStatus::Blocked
        } else {
            IntentCaseTestArtifactStatus::Observed
        }
    } else {
        match (phase, runtime_repair_receipt) {
            (IntentCaseTestArtifactPhase::After, Some(RuntimeRepairCaseReceipt::Live(receipt)))
                if receipt.verification_success =>
            {
                IntentCaseTestArtifactStatus::Verified
            }
            (IntentCaseTestArtifactPhase::After, Some(RuntimeRepairCaseReceipt::Live(_))) => {
                IntentCaseTestArtifactStatus::Failed
            }
            (_, Some(RuntimeRepairCaseReceipt::NoLive(_))) => IntentCaseTestArtifactStatus::Blocked,
            _ => IntentCaseTestArtifactStatus::NotRun,
        }
    }
}
