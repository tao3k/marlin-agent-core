// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Resident single-threaded Gerbil Scheme Bridge Actor (`SchemeBridgeActor`)
//! implementing `marlin_agent_app_server::mygo_host::SchemeBridgeBackend`.
//!
//! Runs strictly on the OS thread that owns the embedded Gambit/Gerbil VM and
//! resolves TWO generic Scheme entrypoints ONCE at startup:
//!   - `:marlin-agent-core/ui-bridge/funcs#marlin-scheme-bridge-list-namespaces`
//!   - `:marlin-agent-core/ui-bridge/funcs#marlin-scheme-bridge-dispatch`
//!
//! Scheme <-> Rust type conversion flows exclusively through the shared
//! `scheme_types` native projection layer (`GerbilSchemeValue::to_rooted_object`
//! and `project_rooted_scheme_object`) — zero manual `decode_*` helpers, zero
//! JSON/text serialization, and zero per-package Rust bridge files.

#![cfg(not(feature = "external-program"))]

use std::{
    collections::HashMap,
    sync::{Arc, mpsc},
};

use gerbil_scheme::{GerbilRuntime, IdentifierKind, RootedSchemeObject};
use marlin_agent_app_server::{
    mygo_host::SchemeBridgeBackend,
    wire::marlin::projection::v1::{GraphRecordView, GraphVisibleSurface},
};
use serde::Deserialize;
use tokio::sync::oneshot;

use crate::scheme_types::{GerbilSchemeValue, project_rooted_scheme_object};

/// Typed Rust projection for the uniform 8-slot Scheme bridge record vector:
/// `#(record-id kind title summary source-path start-line end-line #(#(k v) ...))`.
#[derive(Debug, Deserialize)]
struct SchemeBridgeRecordRow(
    String,
    String,
    String,
    String,
    String,
    u32,
    u32,
    Vec<(String, String)>,
);

impl From<SchemeBridgeRecordRow> for GraphRecordView {
    fn from(row: SchemeBridgeRecordRow) -> Self {
        let SchemeBridgeRecordRow(
            record_id,
            kind,
            title,
            summary,
            path,
            start_line,
            end_line,
            facets_vec,
        ) = row;
        let facets: HashMap<String, String> = facets_vec.into_iter().collect();
        GraphRecordView {
            record_id,
            surface: GraphVisibleSurface::Workspace as i32,
            kind,
            title,
            summary,
            source_path: if path.is_empty() { None } else { Some(path) },
            start_line: (start_line > 0).then_some(start_line),
            end_line: (end_line > 0).then_some(end_line),
            facets,
        }
    }
}

struct BridgeDispatchReq {
    namespace: String,
    verb: String,
    arg: String,
    payload: String,
    reply: oneshot::Sender<Result<Vec<GraphRecordView>, String>>,
}

/// Async, thread-safe handle registered once into `MyGoBridgeHost::register_scheme_backend`.
#[derive(Clone)]
pub struct SchemeBridgeActorHandle {
    namespaces: Arc<Vec<String>>,
    tx: mpsc::Sender<BridgeDispatchReq>,
}

impl SchemeBridgeBackend for SchemeBridgeActorHandle {
    fn namespaces(&self) -> Vec<String> {
        (*self.namespaces).clone()
    }

    fn dispatch_query(
        &self,
        namespace: &str,
        verb: &str,
        arg: Option<&str>,
        payload: &str,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Vec<GraphRecordView>, String>> + Send + 'static>,
    > {
        let (reply_tx, reply_rx) = oneshot::channel();
        let send_res = self.tx.send(BridgeDispatchReq {
            namespace: namespace.to_owned(),
            verb: verb.to_owned(),
            arg: arg.unwrap_or("").to_owned(),
            payload: payload.to_owned(),
            reply: reply_tx,
        });
        Box::pin(async move {
            send_res.map_err(|_| "resident Scheme bridge actor stopped".to_owned())?;
            reply_rx
                .await
                .map_err(|_| "resident Scheme bridge actor dropped reply".to_owned())?
        })
    }
}

/// Resident Gambit/Gerbil VM loop running on the owning OS thread.
pub struct SchemeBridgeActor<'rt> {
    runtime: &'rt GerbilRuntime,
    dispatch_proc: RootedSchemeObject<'rt>,
    rx: mpsc::Receiver<BridgeDispatchReq>,
}

impl<'rt> SchemeBridgeActor<'rt> {
    /// Resolves the unified Scheme Bridge Hub entrypoints and discovers all
    /// dynamically registered Scheme package namespaces (`org`, `markdown`, `doc`, ...).
    pub fn open(
        runtime: &'rt GerbilRuntime,
        scheme_package_dir: &str,
    ) -> Result<(Self, SchemeBridgeActorHandle), String> {
        let module = runtime
            .intern_identifier(IdentifierKind::Symbol, ":marlin-agent-core/ui-bridge/funcs")
            .map_err(|e| format!("intern ui-bridge module: {e}"))?;
        let list_sym = runtime
            .intern_identifier(
                IdentifierKind::Symbol,
                "marlin-scheme-bridge-list-namespaces",
            )
            .map_err(|e| format!("intern list-namespaces export: {e}"))?;
        let dispatch_sym = runtime
            .intern_identifier(IdentifierKind::Symbol, "marlin-scheme-bridge-dispatch")
            .map_err(|e| format!("intern dispatch export: {e}"))?;

        let list_proc = runtime
            .resolve_package_export(scheme_package_dir, &module, &list_sym)
            .map_err(|e| format!("resolve marlin-scheme-bridge-list-namespaces: {e}"))?;
        let dispatch_proc = runtime
            .resolve_package_export(scheme_package_dir, &module, &dispatch_sym)
            .map_err(|e| format!("resolve marlin-scheme-bridge-dispatch: {e}"))?;

        let dummy = GerbilSchemeValue::boolean(false)
            .to_rooted_object(runtime)
            .map_err(|e| format!("project list-namespaces input: {e}"))?;
        let raw_ns = runtime
            .call_rooted(&list_proc, &dummy)
            .map_err(|e| format!("call marlin-scheme-bridge-list-namespaces: {e}"))?;
        let mut namespaces: Vec<String> = project_rooted_scheme_object(raw_ns)
            .map_err(|e| format!("project bridge namespaces: {e}"))?;
        namespaces.retain(|ns| !ns.is_empty());
        namespaces.sort();

        let (tx, rx) = mpsc::channel();
        let handle = SchemeBridgeActorHandle {
            namespaces: Arc::new(namespaces),
            tx,
        };
        Ok((
            Self {
                runtime,
                dispatch_proc,
                rx,
            },
            handle,
        ))
    }

    /// Runs the resident single-threaded Gambit dispatch loop until all handles
    /// are dropped (or process exit).
    pub fn run_loop(self) {
        while let Ok(req) = self.rx.recv() {
            let res = self.dispatch_once(&req.namespace, &req.verb, &req.arg, &req.payload);
            let _ = req.reply.send(res);
        }
    }

    /// Synchronously invokes `marlin-scheme-bridge-dispatch` on the owning VM thread.
    pub fn dispatch_once(
        &self,
        namespace: &str,
        verb: &str,
        arg: &str,
        payload: &str,
    ) -> Result<Vec<GraphRecordView>, String> {
        let req_vec = GerbilSchemeValue::vector([
            GerbilSchemeValue::text(namespace),
            GerbilSchemeValue::text(verb),
            GerbilSchemeValue::text(arg),
            GerbilSchemeValue::text(payload),
        ])
        .to_rooted_object(self.runtime)
        .map_err(|e| format!("project dispatch request: {e}"))?;

        let raw = self
            .runtime
            .call_rooted(&self.dispatch_proc, &req_vec)
            .map_err(|e| format!("scheme bridge dispatch({namespace}:{verb}): {e}"))?;
        let rows: Vec<SchemeBridgeRecordRow> = project_rooted_scheme_object(raw)
            .map_err(|e| format!("project scheme bridge records: {e}"))?;
        Ok(rows.into_iter().map(GraphRecordView::from).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projects_typed_scheme_value_rows_into_graph_records() {
        let scheme_val = GerbilSchemeValue::vector([GerbilSchemeValue::vector([
            GerbilSchemeValue::text("doc:org:0"),
            GerbilSchemeValue::text("doc.block"),
            GerbilSchemeValue::text("Core Roadmap"),
            GerbilSchemeValue::text(""),
            GerbilSchemeValue::text("README.org"),
            GerbilSchemeValue::integer(1),
            GerbilSchemeValue::integer(3),
            GerbilSchemeValue::vector([
                GerbilSchemeValue::vector([
                    GerbilSchemeValue::text("block_kind"),
                    GerbilSchemeValue::text("heading"),
                ]),
                GerbilSchemeValue::vector([
                    GerbilSchemeValue::text("todo"),
                    GerbilSchemeValue::text("TODO"),
                ]),
            ]),
        ])]);

        let rows: Vec<SchemeBridgeRecordRow> = scheme_val
            .decode()
            .expect("GerbilSchemeValue::decode should project SchemeBridgeRecordRow");
        assert_eq!(rows.len(), 1);
        let rec = GraphRecordView::from(rows.into_iter().next().unwrap());
        assert_eq!(rec.record_id, "doc:org:0");
        assert_eq!(rec.kind, "doc.block");
        assert_eq!(rec.title, "Core Roadmap");
        assert_eq!(rec.source_path.as_deref(), Some("README.org"));
        assert_eq!(rec.start_line, Some(1));
        assert_eq!(rec.end_line, Some(3));
        assert_eq!(rec.facets.get("block_kind").map(String::as_str), Some("heading"));
        assert_eq!(rec.facets.get("todo").map(String::as_str), Some("TODO"));
    }
}
