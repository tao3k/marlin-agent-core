//! Status types for `LoopProgram` runtime side effects and replay policy.

/// Aggregate status for runtime side effects derived from one handoff execution receipt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoopProgramRuntimeSideEffectStatus {
    Empty,
    Completed,
    Partial,
    Skipped,
    Failed,
}

/// Status for one tool-process side effect.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoopProgramToolProcessSideEffectStatus {
    Completed,
    Skipped,
    Failed,
}

/// Status for one file-write side effect.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoopProgramFileWriteSideEffectStatus {
    Completed,
    Denied,
    Failed,
}

/// Derived-session replay policy status after runtime side effects are observed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LoopProgramDerivedSessionPolicyStatus {
    Ready,
    Deferred,
    Blocked,
    MissingDerivedSession,
    ReceiptMismatch,
}

impl LoopProgramDerivedSessionPolicyStatus {
    pub fn allows_replay(&self) -> bool {
        matches!(self, Self::Ready)
    }

    pub fn requires_follow_up(&self) -> bool {
        matches!(self, Self::Deferred)
    }

    pub fn blocks_replay(&self) -> bool {
        matches!(
            self,
            Self::Blocked | Self::MissingDerivedSession | Self::ReceiptMismatch
        )
    }
}
