// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Package graph projection of ASP Rust's typed development evidence.

use std::path::{Path, PathBuf};

use asp_rust::{
    AspRustFinding, AspRustReport, RustDeterminismReadiness, RustDeterminismReadinessObservation,
    RustInvariantCandidate, RustModuleReport,
};
use serde::{Deserialize, Serialize};

/// Counts of the nodes and edges emitted for a package's evidence.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RustEvidenceGraphSummary {
    pub nodes: usize,
    pub edges: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(crate) struct PackageEvidenceGraph {
    schema: &'static str,
    nodes: Vec<PackageEvidenceNode>,
    edges: Vec<PackageEvidenceEdge>,
    pub(crate) summary: RustEvidenceGraphSummary,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "evidence", rename_all = "kebab-case")]
enum PackageEvidenceNode {
    Package(PathBuf),
    Source(RustModuleReport),
    Finding(AspRustFinding),
    Invariant(RustInvariantCandidate),
    Determinism(RustDeterminismReadinessObservation),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
struct EvidenceNodeIndex(usize);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
struct PackageEvidenceEdge {
    package: EvidenceNodeIndex,
    evidence: EvidenceNodeIndex,
}

pub(crate) fn project_package_evidence(
    root: &Path,
    report: &AspRustReport,
    readiness: &RustDeterminismReadiness,
) -> PackageEvidenceGraph {
    let evidence = report
        .modules
        .iter()
        .cloned()
        .map(PackageEvidenceNode::Source)
        .chain(
            report
                .findings
                .iter()
                .cloned()
                .map(PackageEvidenceNode::Finding),
        )
        .chain(
            report
                .invariant_candidates
                .iter()
                .cloned()
                .map(PackageEvidenceNode::Invariant),
        )
        .chain(
            readiness
                .observations
                .iter()
                .cloned()
                .map(PackageEvidenceNode::Determinism),
        );
    let mut nodes: Vec<_> = evidence.collect();
    let package = EvidenceNodeIndex(nodes.len());
    let edges: Vec<_> = (0..nodes.len())
        .map(|index| PackageEvidenceEdge {
            package,
            evidence: EvidenceNodeIndex(index),
        })
        .collect();
    if !edges.is_empty() {
        nodes.push(PackageEvidenceNode::Package(root.to_path_buf()));
    }
    let summary = RustEvidenceGraphSummary {
        nodes: nodes.len(),
        edges: edges.len(),
    };
    PackageEvidenceGraph {
        schema: "marlin.rust-package-evidence-graph.v1",
        nodes,
        edges,
        summary,
    }
}
