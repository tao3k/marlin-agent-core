//! Typed loop graph `IR` emitted by the `Gerbil` control plane.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Compiled graph specification ready for Rust-side validation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CompiledLoopGraph {
    pub graph_id: String,
    pub nodes: Vec<LoopNodeSpec>,
    pub edges: Vec<LoopEdgeSpec>,
}

/// Node specification inside a compiled loop graph.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopNodeSpec {
    pub id: String,
    pub executor: String,
    pub config: BTreeMap<String, String>,
}

/// Directed edge specification between loop graph nodes.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LoopEdgeSpec {
    pub from: String,
    pub to: String,
    pub condition: Option<String>,
}
