// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! The optional Marlin App Server transport for external clients.

#[cfg(feature = "grpc")]
mod grpc;
#[cfg(feature = "grpc")]
pub use grpc::AppServer;
#[cfg(feature = "grpc")]
mod session;
#[cfg(feature = "grpc")]
pub use session::{SessionBinding, SessionOpen};
#[cfg(feature = "grpc")]
pub mod atomic_effect_substrate;
#[cfg(feature = "grpc")]
pub use atomic_effect_substrate::{
    AllowlistSandboxRunner, AtomicSandboxRunner, BoundedTokioSandboxRunner,
    CompositeAtomicEffectExecutor, DenySandboxRunner, StorageCasStore,
};
#[cfg(feature = "grpc")]
pub mod task_actors;
#[cfg(feature = "grpc")]
pub use task_actors::{
    RegisterTaskActorRequest, TaskActorControlPlane, TaskActorPersistenceHandle,
};

#[cfg(feature = "client")]
pub mod v1;

#[cfg(feature = "ui-grpc")]
pub mod wire;

#[cfg(feature = "ui-grpc")]
pub mod ui_grpc;
