//! Native AOT artifact planning for Gerbil runtime C ABI link units.

mod api;
mod config;
mod link;
mod paths;
mod receipt;
mod run;
mod status;

pub use config::{
    GerbilDeckRuntimeNativeAotConfig, GerbilDeckRuntimeNativeAotProfile, GerbilNativeCCompiler,
    GerbilNativeLinkLibrary, GerbilNativeSymbolAuditor,
};
pub use link::{
    GerbilDeckRuntimeNativeCargoDirective, GerbilDeckRuntimeNativeCargoDirectiveKind,
    GerbilDeckRuntimeNativeStaticLinkPlan,
};
pub use receipt::{
    GerbilDeckRuntimeNativeAotBuildReceipt, GerbilDeckRuntimeNativeAotCommandPlan,
    GerbilDeckRuntimeNativeAotCommandReceipt, GerbilDeckRuntimeNativeAotPlan,
    GerbilDeckRuntimeNativeSymbol, GerbilDeckRuntimeNativeSymbolAuditMethod,
};
pub use status::{
    GerbilDeckRuntimeNativeAotBuildStatus, GerbilDeckRuntimeNativeAotStatus,
    GerbilDeckRuntimeNativeStaticLinkStatus,
};
