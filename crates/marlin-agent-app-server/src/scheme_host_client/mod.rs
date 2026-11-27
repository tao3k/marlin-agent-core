// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Thin, typed, Bearer-authenticated HTTP/2 client for `marlin-scheme-host` and
//! `marlin-mygo-host`, unifying all 4 live control-plane services over one shared
//! `tonic::transport::Channel`.

mod client;
mod views;

pub use client::{GrpcCode, GrpcStatus, MarlinSchemeHostClient, parse_bearer_header};
pub use views::{
    CedarProbeResultView, FleetAuditCapsuleView, FleetTaskAction,
    GovernanceAuthoritySnapshotView, GovernanceFactAction, HostStatusSnapshot,
    format_facets_sorted, graph_record_to_json, json_escape, workbench_tab_kind_label,
};
