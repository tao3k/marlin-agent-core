//! Native AOT artifact planning for the Gerbil Deck runtime C ABI.

mod api;
mod config;
mod link;
mod paths;
mod receipt;
mod run;
mod status;

pub use config::{
    GerbilDeckRuntimeNativeAotConfig, GerbilNativeCCompiler, GerbilNativeLinkLibrary,
    GerbilNativeSymbolAuditor,
};
pub use link::{
    GerbilDeckRuntimeNativeCargoDirective, GerbilDeckRuntimeNativeCargoDirectiveKind,
    GerbilDeckRuntimeNativeStaticLinkPlan,
};
pub use receipt::{
    GerbilDeckRuntimeNativeAotBuildReceipt, GerbilDeckRuntimeNativeAotCommandPlan,
    GerbilDeckRuntimeNativeAotCommandReceipt, GerbilDeckRuntimeNativeAotPlan,
    GerbilDeckRuntimeNativeSymbol,
};
pub use status::{
    GerbilDeckRuntimeNativeAotBuildStatus, GerbilDeckRuntimeNativeAotStatus,
    GerbilDeckRuntimeNativeStaticLinkStatus,
};
