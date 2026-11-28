// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Native Scheme tooling projection and minimal HTTP/2 `tonic` gRPC search
//! gateway connecting declared capabilities to Marlin's `ModelToolCatalog`.

pub mod aitia_review_tool;
#[cfg(feature = "grpc")]
mod brute_search;
#[cfg(feature = "grpc")]
pub mod grpc_search;
mod projection;
#[cfg(feature = "grpc")]
mod search_scheduler;

pub use aitia_review_tool::{
    AITIA_REVIEW_TOOL_NAME, AitiaGrpcRawResponse, AitiaMagitFinding, AitiaMagitOverlaySummary,
    AitiaMagitReviewSnapshot, AitiaReviewToolHandler, AitiaSidecarGrpcClient,
    ManagedAitiaSidecarServer,
};
#[cfg(feature = "grpc")]
pub use grpc_search::{
    AsyncSearchDslSessionFuture, AsyncSearchDslSessionRunner, GrpcSearchToolHandler,
    LoopbackSearchGrpcServer, SearchGatewayClient, SearchGatewayIndexHandle, SearchGatewayServer,
    SearchGatewayService, tool_schema_entries_from_declarations,
};
#[cfg(feature = "grpc")]
pub use projection::catalog_from_declarations_with_grpc_search;
pub use projection::{
    SharedWorkspaceSearchIndex, ToolingProjectionError, decode_tooling, project_linked,
};
#[cfg(feature = "grpc")]
pub use search_scheduler::{
    CacheDependencyProfile, FencedCacheLookup, QueryCacheKey, SearchQueryTier,
    SearchSchedulerMetrics,
};
