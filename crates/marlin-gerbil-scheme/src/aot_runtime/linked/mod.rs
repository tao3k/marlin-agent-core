// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Native AOT artifact planning for Gerbil runtime C ABI link units.

mod api;
mod config;
mod link;
mod paths;
mod receipt;
mod run;
mod status;

pub use config::{
    GerbilAotConfig, GerbilAotProfile, GerbilCCompiler, GerbilLinkLibrary, GerbilSymbolAuditor,
};
pub use link::{GerbilAotCargoDirective, GerbilAotCargoDirectiveKind, GerbilAotStaticLinkPlan};
pub use receipt::{
    GerbilAotBuildReceipt, GerbilAotCommandPlan, GerbilAotCommandReceipt, GerbilAotPlan,
    GerbilAotSymbol, GerbilAotSymbolAuditMethod,
};
pub use status::{GerbilAotBuildStatus, GerbilAotStaticLinkStatus, GerbilAotStatus};
