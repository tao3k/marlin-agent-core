// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Typed denials at the plugin release and effect boundaries.

use serde::{Deserialize, Serialize};

use super::CapabilityId;

/// The host reason for refusing a release, activation, or effect.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum PluginDenial {
    InvalidDeclaration,
    UnsupportedContract,
    UnknownPublisher,
    InvalidSignature,
    ConflictingRelease,
    ConflictingRegistration,
    MissingRelease,
    UnregisteredWorkflow,
    InvalidSettings,
    ConflictingSettings,
    PolicyUnavailable,
    MissingCapability(CapabilityId),
    ScopeMismatch,
    StaleActivation,
    ActivationInUse,
    CapacityExceeded,
    HostInitialization,
}
