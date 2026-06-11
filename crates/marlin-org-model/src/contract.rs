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
    pub templates: Vec<OrgContractTemplate>,
    pub query_source: Option<OrgContractSourceSpan>,
    pub expect_source: Option<OrgContractSourceSpan>,
}

/// Template text attached to a contract assertion.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgContractTemplate {
    pub kind: OrgContractTemplateKind,
    pub engine: OrgContractTemplateEngine,
    pub body: String,
    pub source: Option<OrgContractSourceSpan>,
}

/// Template role inside a contract assertion.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum OrgContractTemplateKind {
    Message,
    Fix,
}

/// Named query binding used by a contract assertion.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgContractBinding {
    pub name: String,
    pub query: OrgContractQuery,
}

/// Assertion expectation projected from orgize contracts.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OrgContractExpectation {
    Exists,
    NotExists,
    Count {
        op: OrgContractCompareOp,
        expected: usize,
    },
    Unsupported {
        label: String,
    },
}

impl OrgContractExpectation {
    pub fn unsupported(label: impl Into<String>) -> Self {
        Self::Unsupported {
            label: label.into(),
        }
    }

    pub fn evaluate_count(&self, actual: usize) -> Option<bool> {
        match self {
            Self::Exists => Some(actual > 0),
            Self::NotExists => Some(actual == 0),
            Self::Count { op, expected } => Some(op.matches(actual, *expected)),
            Self::Unsupported { .. } => None,
        }
    }

    pub fn expected_summary(&self) -> String {
        match self {
            Self::Exists => "exists".to_string(),
            Self::NotExists => "not exists".to_string(),
            Self::Count { op, expected } => format!("count {} {}", op.as_str(), expected),
            Self::Unsupported { label } => label.clone(),
        }
    }
}

/// Comparison operator for a count-based contract expectation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum OrgContractCompareOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl OrgContractCompareOp {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Eq => "==",
            Self::Ne => "!=",
            Self::Lt => "<",
            Self::Le => "<=",
            Self::Gt => ">",
            Self::Ge => ">=",
        }
    }

    pub fn matches(self, actual: usize, expected: usize) -> bool {
        match self {
            Self::Eq => actual == expected,
            Self::Ne => actual != expected,
            Self::Lt => actual < expected,
            Self::Le => actual <= expected,
            Self::Gt => actual > expected,
            Self::Ge => actual >= expected,
        }
    }
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

/// Runtime-independent validation receipts for contract assertions.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgContractValidationReport {
    pub receipts: Vec<OrgContractValidationReceipt>,
    pub diagnostics: Vec<OrgContractDiagnostic>,
}

/// Result of evaluating one assertion against a document or subtree.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct OrgContractValidationReceipt {
    pub contract_id: OrgContractId,
    pub assertion_id: String,
    pub target: OrgContractValidationTarget,
    pub status: OrgContractValidationStatus,
    pub severity: OrgContractSeverity,
    pub message: Option<String>,
    #[serde(default)]
    pub matched_nodes: Vec<OrgNodeId>,
    pub source: Option<OrgSourceSpan>,
}

/// Target validated by a contract assertion.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum OrgContractValidationTarget {
    Document,
    Node(OrgNodeId),
}

/// Assertion validation outcome.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum OrgContractValidationStatus {
    Passed,
    Failed,
    Skipped,
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
contract_text_id!(OrgContractTemplateEngine);
contract_text_id!(OrgContractElementCategory);
contract_text_id!(OrgContractElementKind);
contract_text_id!(OrgContractRelativeScope);
