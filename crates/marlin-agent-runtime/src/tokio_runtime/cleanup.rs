//! Runtime-owned process cleanup driver integration.

use super::{RuntimeContext, TokioAgentRuntime};
use crate::observability;

impl TokioAgentRuntime {
    pub fn sweep_process_cleanup<C>(
        &self,
        controller: &mut C,
        observed_at_ms: u64,
    ) -> observability::RuntimeCommandCleanupReceipt
    where
        C: observability::RuntimeProcessCleanupController,
    {
        self.process_registry()
            .sweep_cleanup_candidates(controller, observed_at_ms)
            .command_receipt_with_policy(self.process_cleanup_policy())
    }

    pub fn sweep_process_cleanup_with_sysinfo(
        &self,
        observed_at_ms: u64,
    ) -> observability::RuntimeCommandCleanupReceipt {
        let mut controller = observability::SysinfoRuntimeProcessController::new();
        self.sweep_process_cleanup(&mut controller, observed_at_ms)
    }
}

impl RuntimeContext {
    pub fn sweep_process_cleanup<C>(
        &self,
        controller: &mut C,
        observed_at_ms: u64,
    ) -> observability::RuntimeCommandCleanupReceipt
    where
        C: observability::RuntimeProcessCleanupController,
    {
        self.process_registry()
            .sweep_cleanup_candidates(controller, observed_at_ms)
            .command_receipt_with_policy(self.process_cleanup_policy())
    }

    pub fn sweep_process_cleanup_with_sysinfo(
        &self,
        observed_at_ms: u64,
    ) -> observability::RuntimeCommandCleanupReceipt {
        let mut controller = observability::SysinfoRuntimeProcessController::new();
        self.sweep_process_cleanup(&mut controller, observed_at_ms)
    }
}
