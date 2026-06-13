//! Native ABI requirements for graph-loop strategy planes.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

/// Stable identifier for a native ABI required by a graph-loop strategy plane.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GraphNativeAbiId(String);

impl GraphNativeAbiId {
    /// Creates a native ABI identifier.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the native ABI identifier as a string slice.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<String> for GraphNativeAbiId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for GraphNativeAbiId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Stable native symbol required by a graph-loop strategy plane.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(transparent)]
pub struct GraphNativeSymbol(String);

impl GraphNativeSymbol {
    /// Creates a native symbol identifier.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the native symbol as a string slice.
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<String> for GraphNativeSymbol {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for GraphNativeSymbol {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

/// Native ABI requirement attached to a graph-loop strategy proposal.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GraphNativeAbiRequirement {
    pub abi_id: GraphNativeAbiId,
    pub version: u32,
    pub required_symbols: Vec<GraphNativeSymbol>,
}

impl GraphNativeAbiRequirement {
    /// Creates a native ABI requirement with no required symbols yet.
    pub fn new(abi_id: impl Into<GraphNativeAbiId>, version: u32) -> Self {
        Self {
            abi_id: abi_id.into(),
            version,
            required_symbols: Vec::new(),
        }
    }

    /// Adds required native symbols to the ABI requirement.
    pub fn with_required_symbols<I, S>(mut self, symbols: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<GraphNativeSymbol>,
    {
        self.required_symbols
            .extend(symbols.into_iter().map(Into::into));
        self
    }
}

pub(super) fn validate_graph_native_abi_requirement(
    native_abi: &GraphNativeAbiRequirement,
    diagnostics: &mut Vec<String>,
) {
    if native_abi.abi_id.as_str().trim().is_empty() {
        diagnostics.push("graph_policy_proposal.native_abi_id_empty".to_string());
    }
    if native_abi.version == 0 {
        diagnostics.push("graph_policy_proposal.native_abi_version_zero".to_string());
    }
    if native_abi.required_symbols.is_empty() {
        diagnostics.push("graph_policy_proposal.native_abi_symbols_empty".to_string());
    }

    let mut symbols = BTreeSet::new();
    for symbol in &native_abi.required_symbols {
        let symbol = symbol.as_str().trim();
        if symbol.is_empty() {
            diagnostics.push("graph_policy_proposal.native_abi_symbol_empty".to_string());
            continue;
        }
        if !symbols.insert(symbol.to_string()) {
            diagnostics.push(format!(
                "graph_policy_proposal.native_abi_symbol_duplicate:{symbol}"
            ));
        }
    }
}
