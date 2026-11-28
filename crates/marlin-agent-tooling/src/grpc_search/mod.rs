// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Minimal HTTP/2 `tonic` gRPC transport (`marlin.search.v1.SearchGateway`) and
//! razor-thin `ToolRuntime` client replacing ad hoc cross-layer search glue.

mod client;
mod frontier;
pub mod proto;
mod service;

pub use client::{
    GrpcSearchToolHandler, LoopbackSearchGrpcServer, tool_schema_entries_from_declarations,
};
pub use proto::search_gateway_client::SearchGatewayClient;
pub use proto::search_gateway_server::SearchGatewayServer;
pub use proto::{
    EvidenceFrontier, GetToolSchemaRequest, GetToolSchemaResponse, ResetSessionRequest,
    ResetSessionResponse, SearchStepRequest, SearchStepResponse, ToolSchemaEntry,
};
pub use service::{
    AsyncSearchDslSessionFuture, AsyncSearchDslSessionRunner, SearchGatewayIndexHandle,
    SearchGatewayService,
};
