//! Stable Org contract records projected from parser-owned contract facts.

use serde::{Deserialize, Serialize};

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
    pub id: String,
    pub aliases: Vec<String>,
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

contract_text_id!(OrgContractScope);
contract_text_id!(OrgContractKind);
contract_text_id!(OrgContractSeverity);
contract_text_id!(OrgContractExpectation);
contract_text_id!(OrgContractElementCategory);
contract_text_id!(OrgContractElementKind);
contract_text_id!(OrgContractRelativeScope);
