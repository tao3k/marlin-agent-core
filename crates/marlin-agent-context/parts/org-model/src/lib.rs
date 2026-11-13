// SPDX-FileCopyrightText: 2026 tao3k team and Contributors
//
// SPDX-License-Identifier: Apache-2.0 AND LGPL-2.1-or-later

//! Structured `Org` model for workspace records.

mod block;
mod contract;
mod link;
mod node;
mod table;
mod task;

pub use block::{BlockKind, OrgBlock};
pub use contract::{
    OrgContract, OrgContractAssertion, OrgContractBinding, OrgContractCompareOp,
    OrgContractDiagnostic, OrgContractDiagnosticSeverity, OrgContractDocumentPredicate,
    OrgContractElementCategory, OrgContractElementKind, OrgContractExpectation, OrgContractId,
    OrgContractKind, OrgContractPredicate, OrgContractQuery, OrgContractReference,
    OrgContractReferenceScope, OrgContractRegistry, OrgContractRelativeScope,
    OrgContractResolution, OrgContractResolutionReport, OrgContractScope, OrgContractSeverity,
    OrgContractSourceSpan, OrgContractTemplate, OrgContractTemplateEngine, OrgContractTemplateKind,
    OrgContractValidationReceipt, OrgContractValidationReport, OrgContractValidationSkipReason,
    OrgContractValidationStatus, OrgContractValidationTarget, OrgContractValue,
};
pub use link::{LinkKind, OrgLink};
pub use node::{
    OrgNode, OrgNodeId, OrgNodeKind, OrgNodeSourceTokens, OrgProperty, OrgSourceSpan, OrgTimestamp,
};
pub use table::{OrgTable, OrgTableRow};
pub use task::{CheckboxState, OrgCheckbox, TodoState};
