//! Typed LoopProgram projection for Scheme vertical case-driver receipts.

use std::{error::Error, fmt};

use marlin_agent_protocol::{
    LoopMechanismPolicyId, LoopPolicyDigest, LoopPolicyEpoch, LoopProgram, LoopProgramActionKind,
    LoopProgramEventKind, LoopProgramId, LoopProgramInput, LoopProgramStateId,
    LoopProgramTransition, LoopProgramTransitionId,
};

use super::GerbilLoopCaseDriverVerticalTraceReceipt;

/// Error returned when a Scheme vertical receipt cannot become a LoopProgram.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilLoopCaseDriverLoopProgramProjectionError {
    message: String,
}

impl GerbilLoopCaseDriverLoopProgramProjectionError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for GerbilLoopCaseDriverLoopProgramProjectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for GerbilLoopCaseDriverLoopProgramProjectionError {}

/// Project a verified Scheme vertical trace receipt into a Rust LoopProgram.
pub fn project_gerbil_loop_case_driver_loop_program(
    receipt: &GerbilLoopCaseDriverVerticalTraceReceipt,
) -> Result<LoopProgram, GerbilLoopCaseDriverLoopProgramProjectionError> {
    let actions = receipt
        .transition_actions()
        .map(project_gerbil_loop_case_driver_loop_action_kind)
        .collect::<Result<Vec<_>, _>>()?;
    let events = receipt
        .transition_events()
        .map(project_gerbil_loop_case_driver_loop_event_kind)
        .collect::<Result<Vec<_>, _>>()?;
    let policy_digest = <[u8; 32]>::try_from(receipt.policy_digest_octets()).map_err(|_| {
        GerbilLoopCaseDriverLoopProgramProjectionError::new(format!(
            "Scheme-projected policy digest for {} must be exactly 32 octets",
            receipt.loop_program_id().as_str()
        ))
    })?;

    Ok(LoopProgram::new(LoopProgramInput {
        program_id: LoopProgramId::new(receipt.loop_program_id().as_str()),
        policy_epoch: LoopPolicyEpoch::new(receipt.policy_epoch()),
        policy_digest: LoopPolicyDigest::from_bytes(policy_digest),
        mechanism_policies: receipt
            .mechanism_policy_ids()
            .map(LoopMechanismPolicyId::new)
            .collect::<Vec<_>>()
            .into_boxed_slice(),
        initial_state: LoopProgramStateId::new("scheme-state-0"),
        transitions: actions
            .into_iter()
            .zip(events)
            .enumerate()
            .map(|(index, (action, event))| LoopProgramTransition {
                transition_id: LoopProgramTransitionId::new(format!(
                    "scheme-projected-transition-{index}"
                )),
                from: LoopProgramStateId::new(format!("scheme-state-{index}")),
                event,
                action,
                to: LoopProgramStateId::new(format!("scheme-state-{}", index + 1)),
            })
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    }))
}

/// Project a Scheme transition action token into the Rust LoopProgram action enum.
pub fn project_gerbil_loop_case_driver_loop_action_kind(
    action: &str,
) -> Result<LoopProgramActionKind, GerbilLoopCaseDriverLoopProgramProjectionError> {
    match action {
        "continue" => Ok(LoopProgramActionKind::Continue),
        "dispatch_tools" => Ok(LoopProgramActionKind::DispatchTools),
        "invoke_model" => Ok(LoopProgramActionKind::InvokeModel),
        "read_memory" => Ok(LoopProgramActionKind::ReadMemory),
        "rewrite_graph" => Ok(LoopProgramActionKind::RewriteGraph),
        "runtime_handoff" => Ok(LoopProgramActionKind::RuntimeHandoff),
        "stop" => Ok(LoopProgramActionKind::Stop),
        "verify" => Ok(LoopProgramActionKind::Verify),
        action => Err(GerbilLoopCaseDriverLoopProgramProjectionError::new(
            format!("unsupported Scheme-projected loop action {action}"),
        )),
    }
}

/// Project a Scheme transition event token into the Rust LoopProgram event enum.
pub fn project_gerbil_loop_case_driver_loop_event_kind(
    event: &str,
) -> Result<LoopProgramEventKind, GerbilLoopCaseDriverLoopProgramProjectionError> {
    match event {
        "error" => Ok(LoopProgramEventKind::Error),
        "model_event" => Ok(LoopProgramEventKind::ModelEvent),
        "runtime_receipt" => Ok(LoopProgramEventKind::RuntimeReceipt),
        "start" => Ok(LoopProgramEventKind::Start),
        "tool_receipt" => Ok(LoopProgramEventKind::ToolReceipt),
        "tool_request" => Ok(LoopProgramEventKind::ToolRequest),
        "verification_receipt" => Ok(LoopProgramEventKind::VerificationReceipt),
        event => Err(GerbilLoopCaseDriverLoopProgramProjectionError::new(
            format!("unsupported Scheme-projected loop event {event}"),
        )),
    }
}
