//! Small graph builders for harness-owned scenario tests.

use marlin_agent_protocol::{LoopEdgeSpec, LoopGraph, LoopNodeSpec};

/// Builder for compact graph-loop fixtures used by harness scenarios.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HarnessGraphBuilder {
    graph_id: String,
    nodes: Vec<LoopNodeSpec>,
    edges: Vec<LoopEdgeSpec>,
}

impl HarnessGraphBuilder {
    pub fn new(graph_id: impl Into<String>) -> Self {
        Self {
            graph_id: graph_id.into(),
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    pub fn node(mut self, node: impl Into<String>, executor: impl Into<String>) -> Self {
        self.nodes.push(LoopNodeSpec {
            id: node.into(),
            executor: executor.into(),
            config: Default::default(),
        });
        self
    }

    pub fn edge(mut self, from: impl Into<String>, to: impl Into<String>) -> Self {
        self.edges.push(LoopEdgeSpec {
            from: from.into(),
            to: to.into(),
            condition: None,
        });
        self
    }

    pub fn linear<I, Node, Executor>(mut self, nodes: I) -> Self
    where
        I: IntoIterator<Item = (Node, Executor)>,
        Node: Into<String>,
        Executor: Into<String>,
    {
        let mut previous = None;

        for (node, executor) in nodes {
            let node = node.into();
            if let Some(from) = previous.replace(node.clone()) {
                self = self.edge(from, node.clone());
            }
            self = self.node(node, executor);
        }

        self
    }

    pub fn build(self) -> LoopGraph {
        LoopGraph {
            graph_id: self.graph_id,
            nodes: self.nodes,
            edges: self.edges,
        }
    }
}
