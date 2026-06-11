//! Rendered workspace view records.

use marlin_org_model::{
    OrgContractDiagnostic, OrgContractRegistry, OrgContractResolution, OrgContractTemplate,
    OrgContractValidationReport, OrgContractValidationStatus, OrgNodeId,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

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
    #[serde(default)]
    pub registry: OrgContractRegistry,
    pub resolutions: Vec<OrgContractResolution>,
    pub diagnostics: Vec<OrgContractDiagnostic>,
    pub templates: Vec<OrgContractTemplate>,
    pub validations: OrgContractValidationReport,
    pub summary: RenderedContractSummary,
    pub rendered_lines: Vec<String>,
}

/// Named input for rendering selected contract facts into a workspace view.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct RenderedContractFactsInput {
    #[serde(default)]
    pub registry: OrgContractRegistry,
    pub resolutions: Vec<OrgContractResolution>,
    pub diagnostics: Vec<OrgContractDiagnostic>,
    pub templates: Vec<OrgContractTemplate>,
    pub validations: OrgContractValidationReport,
}

impl RenderedContractFacts {
    pub fn new(
        resolutions: Vec<OrgContractResolution>,
        diagnostics: Vec<OrgContractDiagnostic>,
        templates: Vec<OrgContractTemplate>,
        validations: OrgContractValidationReport,
    ) -> Self {
        Self::from_input(RenderedContractFactsInput {
            registry: OrgContractRegistry::default(),
            resolutions,
            diagnostics,
            templates,
            validations,
        })
    }

    pub fn from_input(input: RenderedContractFactsInput) -> Self {
        let mut facts = Self {
            registry: input.registry,
            resolutions: input.resolutions,
            diagnostics: input.diagnostics,
            templates: input.templates,
            validations: input.validations,
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
    #[serde(default)]
    pub contract_assertions: usize,
    pub diagnostics: usize,
    pub templates: usize,
    pub validation_receipts: usize,
    pub validation_passed: usize,
    pub validation_failed: usize,
    pub validation_skipped: usize,
    pub validation_matched_nodes: usize,
    pub validation_matched_node_ids: Vec<OrgNodeId>,
    #[serde(default)]
    pub contract_expectation_summaries: Vec<String>,
    #[serde(default)]
    pub validation_skip_reasons: Vec<String>,
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
        let validation_matched_node_ids = facts
            .validations
            .receipts
            .iter()
            .flat_map(|receipt| receipt.matched_nodes.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let contract_expectation_summaries = facts
            .registry
            .contracts
            .iter()
            .flat_map(|contract| {
                contract.assertions.iter().map(|assertion| {
                    format!(
                        "{}/{}: {}",
                        contract.id.as_str(),
                        assertion.id,
                        assertion.expectation.expected_summary()
                    )
                })
            })
            .collect::<Vec<_>>();
        let validation_skip_reasons = facts
            .validations
            .receipts
            .iter()
            .filter_map(|receipt| {
                receipt.skip_reason.as_ref().map(|reason| {
                    format!(
                        "{}/{}: {}",
                        receipt.contract_id.as_str(),
                        receipt.assertion_id,
                        reason.summary()
                    )
                })
            })
            .collect::<Vec<_>>();

        Self {
            resolved_references,
            unresolved_references: facts.resolutions.len().saturating_sub(resolved_references),
            contract_assertions: contract_expectation_summaries.len(),
            diagnostics: facts.diagnostics.len(),
            templates: facts.templates.len(),
            validation_receipts: facts.validations.receipts.len(),
            validation_passed,
            validation_failed,
            validation_skipped,
            validation_matched_nodes: validation_matched_node_ids.len(),
            validation_matched_node_ids,
            contract_expectation_summaries,
            validation_skip_reasons,
        }
    }

    fn render_lines(&self, diagnostics: &[OrgContractDiagnostic]) -> Vec<String> {
        let mut lines = vec![
            format!("contracts.resolved: {}", self.resolved_references),
            format!("contracts.unresolved: {}", self.unresolved_references),
            format!("contracts.assertions: {}", self.contract_assertions),
            format!("contracts.diagnostics: {}", self.diagnostics),
            format!("contracts.templates: {}", self.templates),
            format!(
                "contracts.validation_receipts: {}",
                self.validation_receipts
            ),
            format!("contracts.validation.passed: {}", self.validation_passed),
            format!("contracts.validation.failed: {}", self.validation_failed),
            format!("contracts.validation.skipped: {}", self.validation_skipped),
            format!(
                "contracts.validation.matched_nodes: {}",
                self.validation_matched_nodes
            ),
        ];

        for node in &self.validation_matched_node_ids {
            lines.push(format!(
                "contract.validation.matched_node: {}",
                node.as_str()
            ));
        }

        for expectation in &self.contract_expectation_summaries {
            lines.push(format!("contract.validation.expectation: {expectation}"));
        }

        for skip_reason in &self.validation_skip_reasons {
            lines.push(format!("contract.validation.skip_reason: {skip_reason}"));
        }

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
