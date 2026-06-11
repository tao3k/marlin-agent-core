//! Stable Org contract records projected from parser-owned contract facts.

use serde::{Deserialize, Serialize};

use crate::node::{OrgNodeId, OrgSourceSpan};

macro_rules! contract_text_id {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self::new(value)
            }
        }

        impl From<String> for $name {
            fn from(value: String) -> Self {
                Self::new(value)
            }
        }
    };
}

/// Parser-owned contract registry attached to an Org document.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgContractRegistry {
    pub contracts: Vec<OrgContract>,
}

/// Contract that can validate or template a document or subtree.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgContract {
    pub id: OrgContractId,
    pub aliases: Vec<OrgContractId>,
    pub scope: OrgContractScope,
    pub kind: OrgContractKind,
    pub assertions: Vec<OrgContractAssertion>,
}

/// One contract assertion extracted from an Org contract definition.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgContractAssertion {
    pub id: String,
    pub severity: OrgContractSeverity,
    pub bindings: Vec<OrgContractBinding>,
    pub query: OrgContractQuery,
    pub expectation: OrgContractExpectation,
    pub message: Option<String>,
    pub fix: Option<String>,
    pub query_source: Option<OrgContractSourceSpan>,
    pub expect_source: Option<OrgContractSourceSpan>,
}

/// Named query binding used by a contract assertion.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgContractBinding {
    pub name: String,
    pub query: OrgContractQuery,
}

/// Query shape used by parser-owned Org contracts.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgContractQuery {
    pub category: Option<OrgContractElementCategory>,
    pub kind: Option<OrgContractElementKind>,
    pub affiliated_name: Option<String>,
    pub context: Option<String>,
    pub outline_path_prefix: Vec<String>,
    pub outline_path_exact_len: Option<usize>,
    pub property_equals: Vec<(String, String)>,
    pub property_contains: Vec<(String, String)>,
    pub summary_equals: Vec<(String, String)>,
    pub summary_contains: Vec<(String, String)>,
    pub limit: Option<usize>,
    pub use_scope_outline_path: bool,
    pub has_outline_path_prefix: bool,
    pub scope_outline_depth: Option<usize>,
    pub relative_to: Option<OrgContractRelativeScope>,
}

/// Source block span for a contract query or expectation block.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgContractSourceSpan {
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

/// Resolved and unresolved `CONTRACT_ORG` references found in a document.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgContractResolutionReport {
    pub references: Vec<OrgContractResolution>,
    pub diagnostics: Vec<OrgContractDiagnostic>,
}

/// One `CONTRACT_ORG` reference projected from a document or subtree.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgContractReference {
    pub raw: String,
    pub path: Option<String>,
    pub contract_id: Option<OrgContractId>,
    pub scope: OrgContractReferenceScope,
    pub target_node: Option<OrgNodeId>,
    pub source: Option<OrgSourceSpan>,
}

/// Resolution result for one contract reference.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgContractResolution {
    pub reference: OrgContractReference,
    pub resolved_contract_id: Option<OrgContractId>,
}

/// Diagnostic produced while resolving contract references.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgContractDiagnostic {
    pub code: String,
    pub severity: OrgContractDiagnosticSeverity,
    pub message: String,
    pub reference: OrgContractReference,
}

/// Scope where a `CONTRACT_ORG` reference was found.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum OrgContractReferenceScope {
    Document,
    Subtree,
}

/// Severity for contract reference resolution diagnostics.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum OrgContractDiagnosticSeverity {
    Warning,
    Error,
}

contract_text_id!(OrgContractId);
contract_text_id!(OrgContractScope);
contract_text_id!(OrgContractKind);
contract_text_id!(OrgContractSeverity);
contract_text_id!(OrgContractExpectation);
contract_text_id!(OrgContractElementCategory);
contract_text_id!(OrgContractElementKind);
contract_text_id!(OrgContractRelativeScope);
