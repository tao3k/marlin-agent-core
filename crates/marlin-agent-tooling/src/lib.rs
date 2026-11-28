// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Native Scheme tooling projection and reusable resident-search tool bindings
//! connecting declared capabilities to Marlin's `ModelToolCatalog`.

mod brute_search;
#[cfg(feature = "grpc")]
pub mod grpc_search;
mod projection;
mod search_scheduler;
mod search_tool;

#[cfg(feature = "grpc")]
pub use grpc_search::{
    GrpcSearchToolHandler, LoopbackSearchGrpcServer, SearchGatewayClient, SearchGatewayServer,
    SearchGatewayService, tool_schema_entries_from_declarations,
};
#[cfg(feature = "grpc")]
pub use projection::catalog_from_declarations_with_grpc_search;
pub use projection::{
    ToolingProjectionError, catalog_from_declarations, catalog_from_declarations_with_search,
    decode_tooling, project_linked,
};
pub use search_scheduler::{
    AsyncSearchDslFuture, AsyncSearchDslRunner, CacheDependencyProfile, FencedCacheLookup,
    QueryCacheKey, SearchQueryTier, SearchSchedulerMetrics,
};
pub use search_tool::{
    ResidentSearchToolHandler, SharedSearchIndexSlot, SharedWorkspaceSearchIndex,
};
