//! Performance budget checks for harness execution reports.

use std::time::Duration;

use marlin_agent_protocol::{LoopEvidence, LoopEvidenceKind};

use crate::runtime::{HarnessExecutionReport, HarnessExecutionSummary};

/// Optional performance limits for one harness execution.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct HarnessPerformanceBudget {
    max_duration: Option<Duration>,
    max_event_count: Option<usize>,
    max_span_count: Option<usize>,
    max_diagnostic_count: Option<usize>,
}

impl HarnessPerformanceBudget {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_max_duration(mut self, max_duration: Duration) -> Self {
        self.max_duration = Some(max_duration);
        self
    }

    pub fn with_max_event_count(mut self, max_event_count: usize) -> Self {
        self.max_event_count = Some(max_event_count);
        self
    }

    pub fn with_max_span_count(mut self, max_span_count: usize) -> Self {
        self.max_span_count = Some(max_span_count);
        self
    }

    pub fn with_max_diagnostic_count(mut self, max_diagnostic_count: usize) -> Self {
        self.max_diagnostic_count = Some(max_diagnostic_count);
        self
    }

    pub fn evaluate_summary(&self, summary: &HarnessExecutionSummary) -> HarnessPerformanceReport {
        let measurements = HarnessPerformanceMeasurements::from_summary(summary);
        let mut diagnostics = Vec::new();

        if let Some(max_duration) = self.max_duration
            && summary.duration > max_duration
        {
            diagnostics.push(format!(
                "duration_ms {} exceeded budget {}",
                duration_ms(summary.duration),
                duration_ms(max_duration)
            ));
        }
        push_count_diagnostic(
            &mut diagnostics,
            "event_count",
            summary.event_count,
            self.max_event_count,
        );
        push_count_diagnostic(
            &mut diagnostics,
            "span_count",
            summary.span_count,
            self.max_span_count,
        );
        push_count_diagnostic(
            &mut diagnostics,
            "diagnostic_count",
            summary.diagnostic_count,
            self.max_diagnostic_count,
        );

        HarnessPerformanceReport {
            accepted: diagnostics.is_empty(),
            measurements,
            diagnostics,
        }
    }

    pub fn evaluate_report(&self, report: &HarnessExecutionReport) -> HarnessPerformanceReport {
        self.evaluate_summary(&report.summary)
    }
}

/// Measured values used for performance budget checks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HarnessPerformanceMeasurements {
    pub duration: Duration,
    pub event_count: usize,
    pub span_count: usize,
    pub diagnostic_count: usize,
}

impl HarnessPerformanceMeasurements {
    pub fn from_summary(summary: &HarnessExecutionSummary) -> Self {
        Self {
            duration: summary.duration,
            event_count: summary.event_count,
            span_count: summary.span_count,
            diagnostic_count: summary.diagnostic_count,
        }
    }
}

/// Result of checking one harness execution against a performance budget.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HarnessPerformanceReport {
    pub accepted: bool,
    pub measurements: HarnessPerformanceMeasurements,
    pub diagnostics: Vec<String>,
}

impl HarnessPerformanceReport {
    pub fn is_success(&self) -> bool {
        self.accepted
    }

    pub fn evidence(&self) -> LoopEvidence {
        let detail = format!(
            "accepted={} duration_ms={} event_count={} span_count={} diagnostic_count={} diagnostics={}",
            self.accepted,
            duration_ms(self.measurements.duration),
            self.measurements.event_count,
            self.measurements.span_count,
            self.measurements.diagnostic_count,
            self.diagnostics.len()
        );

        if self.accepted {
            LoopEvidence::present(LoopEvidenceKind::Budget, "harness-performance")
                .with_detail(detail)
        } else {
            LoopEvidence::missing(LoopEvidenceKind::Budget, "harness-performance")
                .with_detail(detail)
        }
    }
}

fn push_count_diagnostic(
    diagnostics: &mut Vec<String>,
    label: &str,
    measured: usize,
    limit: Option<usize>,
) {
    if let Some(limit) = limit
        && measured > limit
    {
        diagnostics.push(format!("{label} {measured} exceeded budget {limit}"));
    }
}

fn duration_ms(duration: Duration) -> u64 {
    duration.as_millis().min(u128::from(u64::MAX)) as u64
}
