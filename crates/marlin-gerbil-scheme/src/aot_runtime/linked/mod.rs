//! Native AOT artifact planning for Gerbil runtime C ABI link units.

mod api;
mod config;
mod link;
mod paths;
mod receipt;
mod run;
mod status;

pub use config::{
    GerbilCCompiler, GerbilDeckRuntimeAotConfig, GerbilDeckRuntimeAotProfile, GerbilLinkLibrary,
    GerbilSymbolAuditor,
};
pub use link::{
    GerbilDeckRouteAbiCargoDirective, GerbilDeckRouteAbiCargoDirectiveKind,
    GerbilDeckRouteAbiStaticLinkPlan,
};
pub use receipt::{
    GerbilDeckRouteAbiSymbol, GerbilDeckRouteAbiSymbolAuditMethod,
    GerbilDeckRuntimeAotBuildReceipt, GerbilDeckRuntimeAotCommandPlan,
    GerbilDeckRuntimeAotCommandReceipt, GerbilDeckRuntimeAotPlan,
};
pub use status::{
    GerbilDeckRouteAbiStaticLinkStatus, GerbilDeckRuntimeAotBuildStatus, GerbilDeckRuntimeAotStatus,
};
