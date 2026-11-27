// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

#[path = "unit/policy.rs"]
mod policy;
#[cfg(feature = "grpc")]
#[path = "unit/atomic_effect_substrate.rs"]
mod atomic_effect_substrate;
#[cfg(feature = "grpc")]
#[path = "unit/grpc.rs"]
mod grpc;
#[cfg(feature = "grpc")]
#[path = "unit/task_actors.rs"]
mod task_actors;
#[cfg(feature = "ui-grpc")]
#[path = "unit/mygo_host.rs"]
mod mygo_host;
#[cfg(feature = "ui-grpc")]
#[path = "unit/scheme_ctl.rs"]
mod scheme_ctl;
#[cfg(feature = "ui-grpc")]
#[path = "unit/scheme_host_client.rs"]
mod scheme_host_client;
#[cfg(feature = "ui-grpc")]
#[path = "unit/ui_grpc.rs"]
mod ui_grpc;
