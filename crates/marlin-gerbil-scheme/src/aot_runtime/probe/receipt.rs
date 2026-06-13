//! Receipt types returned by Gerbil AOT probe and repair APIs.

use super::status::{
    GerbilAotBackendRepairStatus, GerbilAotBackendShimStatus, GerbilAotProbeStatus,
};
use std::path::PathBuf;

/// Captured command result for a `Gerbil` ahead-of-time compiler step.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilAotCommandReceipt {
    pub status_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

/// Structured result for probing the `Gerbil` ahead-of-time compiler toolchain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilAotProbeReceipt {
    pub status: GerbilAotProbeStatus,
    pub gxc: PathBuf,
    pub gsc: PathBuf,
    pub backend_gsc: Option<PathBuf>,
    pub root: PathBuf,
    pub executable: PathBuf,
    pub detail: Option<String>,
    pub module_compile: Option<GerbilAotCommandReceipt>,
    pub executable_compile: Option<GerbilAotCommandReceipt>,
}

/// Result of preparing a local backend `gsc` shim for a failed AOT probe.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilAotBackendShimReceipt {
    pub status: GerbilAotBackendShimStatus,
    pub gsc: PathBuf,
    pub backend_gsc: Option<PathBuf>,
    pub allowed_root: PathBuf,
    pub detail: Option<String>,
}

/// Dry-run plan for repairing a missing backend `gsc` path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilAotBackendRepairReceipt {
    pub status: GerbilAotBackendRepairStatus,
    pub gsc: PathBuf,
    pub backend_gsc: Option<PathBuf>,
    pub allowed_root: PathBuf,
    pub can_create_shim: bool,
    pub requires_system_write: bool,
    pub recommended_action: String,
    pub detail: Option<String>,
}
