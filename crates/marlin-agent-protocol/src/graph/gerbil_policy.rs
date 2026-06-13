//! Gerbil loop graph `IR` compilation into Rust graph-policy proposals.

use super::{
    GraphLoopStrategy, GraphNativeAbiRequirement, GraphPolicyDigest, GraphPolicyProposal,
    LoopEdgeSpec, LoopGraph, LoopNodeSpec,
};
use serde::{Deserialize, Serialize};

/// Schema identifier for Gerbil loop graph `IR` compilation requests.
pub const GERBIL_LOOP_GRAPH_POLICY_COMPILATION_SCHEMA_ID: &str =
    "marlin.agent.gerbil_loop_graph_policy_compilation.v1";

/// Typed request to compile a Gerbil-emitted loop graph `IR` into a policy proposal.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilLoopGraphPolicyCompilationRequest {
    pub strategy: GraphLoopStrategy,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub native_abi: Option<GraphNativeAbiRequirement>,
    pub compiled_graph: marlin_gerbil_ir::CompiledLoopGraph,
    pub input_digest: GraphPolicyDigest,
    pub output_digest: GraphPolicyDigest,
    pub diagnostics: Vec<String>,
}

impl GerbilLoopGraphPolicyCompilationRequest {
    /// Creates a typed Gerbil `IR` compilation request.
    pub fn new(
        strategy: GraphLoopStrategy,
        compiled_graph: marlin_gerbil_ir::CompiledLoopGraph,
        input_digest: impl Into<GraphPolicyDigest>,
        output_digest: impl Into<GraphPolicyDigest>,
    ) -> Self {
        Self {
            strategy,
            native_abi: None,
            compiled_graph,
            input_digest: input_digest.into(),
            output_digest: output_digest.into(),
            diagnostics: Vec::new(),
        }
    }

    /// Adds one Gerbil strategy diagnostic to the compilation request.
    pub fn with_diagnostic(mut self, diagnostic: impl Into<String>) -> Self {
        self.diagnostics.push(diagnostic.into());
        self
    }

    /// Attaches the native ABI requirement proven by the Gerbil adapter.
    pub fn with_native_abi_requirement(mut self, native_abi: GraphNativeAbiRequirement) -> Self {
        self.native_abi = Some(native_abi);
        self
    }
}

/// Compiles Gerbil-emitted graph `IR` into the Rust protocol graph shape.
pub fn compile_gerbil_loop_graph(compiled_graph: marlin_gerbil_ir::CompiledLoopGraph) -> LoopGraph {
    LoopGraph {
        graph_id: compiled_graph.graph_id,
        nodes: compiled_graph
            .nodes
            .into_iter()
            .map(|node| LoopNodeSpec {
                id: node.id,
                executor: node.executor,
                config: node.config,
            })
            .collect(),
        edges: compiled_graph
            .edges
            .into_iter()
            .map(|edge| LoopEdgeSpec {
                from: edge.from,
                to: edge.to,
                condition: edge.condition,
            })
            .collect(),
    }
}

/// Compiles Gerbil-emitted graph `IR` into a Rust-validatable policy proposal.
pub fn compile_gerbil_loop_graph_policy(
    request: GerbilLoopGraphPolicyCompilationRequest,
) -> GraphPolicyProposal {
    let mut proposal = GraphPolicyProposal::new(
        request.strategy,
        compile_gerbil_loop_graph(request.compiled_graph),
        request.input_digest,
        request.output_digest,
    );
    proposal.native_abi = request.native_abi;
    proposal.diagnostics = request.diagnostics;
    proposal
}
