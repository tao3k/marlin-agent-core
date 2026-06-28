//! Harness report span projection for intent-case artifact bundles.

use crate::runtime::{AgentHarnessExecutionReport, AgentHarnessGraphLoopExecutionReport};
use marlin_agent_harness_types::{IntentCaseArtifactManifest, IntentCaseSpanName};

/// Harness-derived span names observed during an intent-case run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IntentCaseObservedSpanSource {
    span_names: Vec<IntentCaseSpanName>,
}

impl IntentCaseObservedSpanSource {
    #[must_use]
    pub fn new(span_names: impl IntoIterator<Item = impl Into<IntentCaseSpanName>>) -> Self {
        let mut span_names = span_names.into_iter().map(Into::into).collect::<Vec<_>>();
        span_names.sort();
        span_names.dedup();
        Self { span_names }
    }

    #[must_use]
    pub fn from_agent_harness_execution_report(report: &AgentHarnessExecutionReport) -> Self {
        Self::new(report.span_names.iter().map(|span_name| span_name.as_str()))
    }

    #[must_use]
    pub fn from_agent_harness_graph_loop_execution_report(
        report: &AgentHarnessGraphLoopExecutionReport,
    ) -> Self {
        Self::new(report.span_names.iter().map(|span_name| span_name.as_str()))
    }

    #[must_use]
    pub fn span_names(&self) -> &[IntentCaseSpanName] {
        &self.span_names
    }

    #[must_use]
    pub(crate) fn enrich_manifest(
        manifest: IntentCaseArtifactManifest,
        observed_span_source: Option<&Self>,
    ) -> IntentCaseArtifactManifest {
        let Some(observed_span_source) = observed_span_source else {
            return manifest;
        };

        manifest.with_observed_span_names(observed_span_source.span_names().iter().cloned())
    }
}
