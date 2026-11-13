// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! One working-copy-bound input and its runtime environment.

use super::{RuntimeEnvironment, WorkingCopyIsolationReceipt};

/// One working-copy-bound sub-agent spawn in a runtime fanout.
#[derive(Clone, Debug)]
pub struct WorkingCopySubAgentFanoutItem<I> {
    input: I,
    environment: RuntimeEnvironment,
    working_copy_receipt: WorkingCopyIsolationReceipt,
}

impl<I> WorkingCopySubAgentFanoutItem<I> {
    pub fn new(
        input: I,
        environment: RuntimeEnvironment,
        working_copy_receipt: WorkingCopyIsolationReceipt,
    ) -> Self {
        Self {
            input,
            environment,
            working_copy_receipt,
        }
    }

    pub fn input(&self) -> &I {
        &self.input
    }

    pub fn environment(&self) -> &RuntimeEnvironment {
        &self.environment
    }

    pub fn working_copy_receipt(&self) -> &WorkingCopyIsolationReceipt {
        &self.working_copy_receipt
    }

    pub fn into_parts(self) -> (I, RuntimeEnvironment, WorkingCopyIsolationReceipt) {
        (self.input, self.environment, self.working_copy_receipt)
    }
}
