//! Rendered workspace view records.

use marlin_org_model::{
    OrgContractDiagnostic, OrgContractResolution, OrgContractTemplate, OrgContractValidationReport,
    OrgContractValidationStatus, OrgNodeId,
};
use serde::{Deserialize, Serialize};

/// Rendered workspace view bounded for agent or UI consumers.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RenderedWorkspaceView {
    pub spec_hash: String,
    pub token_estimate: usize,
    pub nodes: Vec<RenderedViewNode>,
    pub contract_facts: Option<RenderedContractFacts>,
    pub text: String,
}

/// Contract facts selected for a rendered workspace view.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RenderedContractFacts {
    pub resolutions: Vec<OrgContractResolution>,
    pub diagnostics: Vec<OrgContractDiagnostic>,
    pub templates: Vec<OrgContractTemplate>,
    pub validations: OrgContractValidationReport,
    pub summary: RenderedContractSummary,
    pub rendered_lines: Vec<String>,
}

impl RenderedContractFacts {
    pub fn new(
        resolutions: Vec<OrgContractResolution>,
        diagnostics: Vec<OrgContractDiagnostic>,
        templates: Vec<OrgContractTemplate>,
        validations: OrgContractValidationReport,
    ) -> Self {
        let mut facts = Self {
            resolutions,
            diagnostics,
            templates,
            validations,
            summary: RenderedContractSummary::default(),
            rendered_lines: Vec::new(),
        };
        facts.refresh_summary();
        facts
    }

    pub fn refresh_summary(&mut self) {
        self.summary = RenderedContractSummary::from_facts(self);
        self.rendered_lines = self.summary.render_lines(&self.diagnostics);
    }
}

/// Derived counts for contract facts selected into a workspace view.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RenderedContractSummary {
    pub resolved_references: usize,
    pub unresolved_references: usize,
    pub diagnostics: usize,
    pub templates: usize,
    pub validation_receipts: usize,
    pub validation_passed: usize,
    pub validation_failed: usize,
    pub validation_skipped: usize,
}

impl RenderedContractSummary {
    fn from_facts(facts: &RenderedContractFacts) -> Self {
        let resolved_references = facts
            .resolutions
            .iter()
            .filter(|resolution| resolution.resolved_contract_id.is_some())
            .count();
        let validation_passed = facts
            .validations
            .receipts
            .iter()
            .filter(|receipt| matches!(receipt.status, OrgContractValidationStatus::Passed))
            .count();
        let validation_failed = facts
            .validations
            .receipts
            .iter()
            .filter(|receipt| matches!(receipt.status, OrgContractValidationStatus::Failed))
            .count();
        let validation_skipped = facts
            .validations
            .receipts
            .iter()
            .filter(|receipt| matches!(receipt.status, OrgContractValidationStatus::Skipped))
            .count();

        Self {
            resolved_references,
            unresolved_references: facts.resolutions.len().saturating_sub(resolved_references),
            diagnostics: facts.diagnostics.len(),
            templates: facts.templates.len(),
            validation_receipts: facts.validations.receipts.len(),
            validation_passed,
            validation_failed,
            validation_skipped,
        }
    }

    fn render_lines(&self, diagnostics: &[OrgContractDiagnostic]) -> Vec<String> {
        let mut lines = vec![
            format!("contracts.resolved: {}", self.resolved_references),
            format!("contracts.unresolved: {}", self.unresolved_references),
            format!("contracts.diagnostics: {}", self.diagnostics),
            format!("contracts.templates: {}", self.templates),
            format!(
                "contracts.validation_receipts: {}",
                self.validation_receipts
            ),
            format!("contracts.validation.passed: {}", self.validation_passed),
            format!("contracts.validation.failed: {}", self.validation_failed),
            format!("contracts.validation.skipped: {}", self.validation_skipped),
        ];

        for diagnostic in diagnostics {
            lines.push(format!(
                "contract.diagnostic.{}: {}",
                diagnostic.code, diagnostic.message
            ));
        }

        lines
    }
}

/// Rendered projection of one workspace node.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct RenderedViewNode {
    pub node_id: OrgNodeId,
    pub title: Option<String>,
    pub lines: Vec<String>,
}
