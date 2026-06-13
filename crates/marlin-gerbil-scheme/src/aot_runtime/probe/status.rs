//! Status enums returned by Gerbil AOT probe and repair steps.

use serde::{Deserialize, Serialize};

/// Status reported by a `Gerbil` ahead-of-time compiler probe.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum GerbilAotProbeStatus {
    MissingGxc,
    MissingGsc,
    GscBackendUnavailable,
    AssetWriteFailed,
    ModuleCompileFailed,
    ExecutableCompileFailed,
    ExecutableReady,
}

/// Status reported after attempting to prepare a backend `gsc` shim.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GerbilAotBackendShimStatus {
    NotNeeded,
    MissingBackendPath,
    MissingConfiguredGsc,
    OutsideAllowedRoot,
    AlreadyReady,
    Created,
}

/// Status reported by a dry-run backend `gsc` repair plan.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GerbilAotBackendRepairStatus {
    NotNeeded,
    MissingBackendPath,
    MissingConfiguredGsc,
    AlreadyReady,
    RepoShimAvailable,
    RequiresSystemWrite,
}
