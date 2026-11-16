// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Runtime side-effect execution for typed `LoopProgram` handoff projections.

mod executor;
mod file_write;
mod process;
mod receipt;
mod status;

pub use executor::LoopProgramRuntimeSideEffectExecutor;
pub use file_write::{
    LoopProgramFileSandbox, LoopProgramFileWriteReceipt, LoopProgramFileWriteRequest,
    LoopProgramFileWriteResolver, LoopProgramFileWriteSideEffectReceipt,
    LoopProgramFileWriteTemplate, StaticLoopProgramFileWriteResolver,
};
pub(crate) use file_write::{read_existing_digest, stable_bytes_digest};
pub use process::{
    LoopProgramToolProcessCommandTemplate, LoopProgramToolProcessResolver,
    LoopProgramToolProcessSideEffectReceipt, StaticLoopProgramToolProcessResolver,
};
pub use receipt::{
    LoopProgramExecutionReplayBundleReceipt, LoopProgramRuntimeReplayBundleReceipt,
    LoopProgramRuntimeSideEffectReceipt,
};
pub use status::{
    LoopProgramDerivedSessionPolicyStatus, LoopProgramFileWriteSideEffectStatus,
    LoopProgramRuntimeSideEffectStatus, LoopProgramToolProcessSideEffectStatus,
};
