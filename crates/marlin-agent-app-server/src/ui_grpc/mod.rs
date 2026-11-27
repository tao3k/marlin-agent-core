// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Rust-owned `ui-grpc` host bridging `marlin-agent-runtime` (signed unified
//! panel releases, capability grants) and Scheme `ui-mygo` overlays to the 6
//! MyGo gRPC services (`marlin.{app,ui,gate,plugin,device,projection}.v1`).

mod device_projection_app;
mod host;
mod plugin_service;
mod state;
mod workbench_gate;

pub use host::MyGoBridgeHost;
pub use state::{
    ClaimedGraphHumanDecision, DispatchedUiCommand, GraphQueryHandler, SchemeOverlayProjection,
};
