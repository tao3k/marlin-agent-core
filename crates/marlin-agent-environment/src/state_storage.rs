//! Initializes runtime-owned state-home directories.

use std::fs;

use marlin_agent_protocol::{
    RuntimeStateLayout, RuntimeStateStorageReceipt, RuntimeStateStorageStatus,
};

/// Filesystem initializer for the unified runtime state home.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RuntimeStateStorageInitializer;

impl RuntimeStateStorageInitializer {
    pub fn new() -> Self {
        Self
    }

    pub fn ensure_layout(&self, layout: &RuntimeStateLayout) -> RuntimeStateStorageReceipt {
        for directory in &layout.directories {
            if let Err(error) = fs::create_dir_all(&directory.path) {
                return RuntimeStateStorageReceipt::failed(
                    layout.home.clone(),
                    format!(
                        "failed to create {} state directory at {}: {error}",
                        directory.kind.relative_path(),
                        directory.path.display()
                    ),
                );
            }
        }

        let receipt = RuntimeStateStorageReceipt::initialized(layout);
        debug_assert_eq!(receipt.status, RuntimeStateStorageStatus::Initialized);
        receipt
    }
}
