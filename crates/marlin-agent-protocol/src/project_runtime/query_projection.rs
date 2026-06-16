//! Typed projections from runtime receipts into project graph query responses.

use crate::{
    graph::{FailureClassificationReceipt, GraphLoopIterationReport, GraphNodeExecutionReceipt},
    trace::AgentExecutionTrace,
};

use super::query::{
    GraphQueryContext, GraphQueryFamily, GraphQueryMatch, GraphQueryRequest, GraphQueryResponse,
};

impl GraphQueryResponse {
    pub fn from_failure_classification_receipt(
        receipt_id: impl Into<String>,
        context: GraphQueryContext,
        receipt: FailureClassificationReceipt,
    ) -> Self {
        let query = failure_classification_query_text(&receipt);
        Self::from_failure_classification_receipts(receipt_id, context, query, vec![receipt])
    }

    pub fn from_failure_classification_receipts(
        receipt_id: impl Into<String>,
        context: GraphQueryContext,
        query: impl Into<String>,
        receipts: impl IntoIterator<Item = FailureClassificationReceipt>,
    ) -> Self {
        let source_project_id = context.project_id.as_str().to_owned();
        let request = GraphQueryRequest::new(context, GraphQueryFamily::Failure, query);
        receipts
            .into_iter()
            .fold(Self::new(receipt_id, request), |response, receipt| {
                response.with_match(failure_classification_query_match(
                    &source_project_id,
                    &receipt,
                ))
            })
    }

    pub fn from_iteration_reports_failure(
        receipt_id: impl Into<String>,
        context: GraphQueryContext,
        query: impl Into<String>,
        reports: impl IntoIterator<Item = GraphLoopIterationReport>,
    ) -> Self {
        let receipts = reports
            .into_iter()
            .filter_map(|report| report.failure_classification_receipt);
        Self::from_failure_classification_receipts(receipt_id, context, query, receipts)
    }

    pub fn from_iteration_reports_evidence(
        receipt_id: impl Into<String>,
        context: GraphQueryContext,
        query: impl Into<String>,
        reports: impl IntoIterator<Item = GraphLoopIterationReport>,
    ) -> Self {
        let source_project_id = context.project_id.as_str().to_owned();
        let request = GraphQueryRequest::new(context, GraphQueryFamily::Evidence, query);
        reports
            .into_iter()
            .fold(Self::new(receipt_id, request), |mut response, report| {
                let run_id = report.execution_result.snapshot.run_id.clone();
                if let Some(trace) = report.trace.as_ref() {
                    response = response.with_match(trace_evidence_query_match(
                        &source_project_id,
                        report.iteration,
                        trace,
                    ));
                }
                for node_receipt in &report.execution_result.node_receipts {
                    response = response.with_match(node_receipt_evidence_query_match(
                        &source_project_id,
                        &run_id,
                        report.iteration,
                        node_receipt,
                    ));
                }
                response
            })
    }
}

fn failure_classification_query_text(receipt: &FailureClassificationReceipt) -> String {
    format!(
        "{:?} {} run:{} iteration:{}",
        receipt.failure_kind,
        receipt.classification_id.as_str(),
        receipt.run_id.as_str(),
        receipt.iteration_id.get()
    )
}

fn failure_classification_query_match(
    source_project_id: &str,
    receipt: &FailureClassificationReceipt,
) -> GraphQueryMatch {
    GraphQueryMatch::new(
        source_project_id,
        failure_classification_summary(receipt),
        failure_classification_score_basis_points(receipt),
    )
    .with_evidence(receipt.classification_id.as_str())
    .with_receipt(receipt.classification_id.as_str())
    .with_source_anchor(format!(
        "graph-loop:{}:iteration:{}",
        receipt.run_id.as_str(),
        receipt.iteration_id.get()
    ))
}

fn failure_classification_summary(receipt: &FailureClassificationReceipt) -> String {
    format!(
        "{:?} for run {} iteration {} retryable={} requires_human={} diagnostics={} source_nodes={}",
        receipt.failure_kind,
        receipt.run_id.as_str(),
        receipt.iteration_id.get(),
        receipt.retryable,
        receipt.requires_human,
        receipt.diagnostics.len(),
        receipt.source_nodes.len()
    )
}

fn failure_classification_score_basis_points(receipt: &FailureClassificationReceipt) -> u16 {
    if receipt.requires_human {
        9_500
    } else if receipt.retryable {
        8_500
    } else {
        8_000
    }
}

fn trace_evidence_query_match(
    source_project_id: &str,
    iteration: u64,
    trace: &AgentExecutionTrace,
) -> GraphQueryMatch {
    GraphQueryMatch::new(
        source_project_id,
        trace_evidence_summary(trace),
        trace_evidence_score_basis_points(trace),
    )
    .with_evidence(format!(
        "trace:{}:iteration:{}",
        trace.run_id.as_str(),
        iteration
    ))
    .with_receipt(format!(
        "trace:{}:iteration:{}",
        trace.run_id.as_str(),
        iteration
    ))
    .with_source_anchor(format!(
        "graph-loop:{}:iteration:{}:trace",
        trace.run_id.as_str(),
        iteration
    ))
}

fn trace_evidence_summary(trace: &AgentExecutionTrace) -> String {
    format!(
        "trace for run {} graph {} status {:?} events={} spans={} diagnostics={}",
        trace.run_id.as_str(),
        trace.graph_id.as_str(),
        trace.status,
        trace.events.len(),
        trace.spans.len(),
        trace.diagnostics.len()
    )
}

fn trace_evidence_score_basis_points(trace: &AgentExecutionTrace) -> u16 {
    if trace.diagnostics.is_empty() {
        8_500
    } else {
        9_000
    }
}

fn node_receipt_evidence_query_match(
    source_project_id: &str,
    run_id: &str,
    iteration: u64,
    receipt: &GraphNodeExecutionReceipt,
) -> GraphQueryMatch {
    GraphQueryMatch::new(
        source_project_id,
        node_receipt_evidence_summary(receipt),
        node_receipt_evidence_score_basis_points(receipt),
    )
    .with_evidence(format!(
        "node-receipt:{}:{}:{}",
        run_id,
        iteration,
        receipt.node_id.as_str()
    ))
    .with_receipt(format!(
        "node-receipt:{}:{}:{}",
        run_id,
        iteration,
        receipt.node_id.as_str()
    ))
    .with_source_anchor(format!(
        "graph-loop:{}:iteration:{}:node:{}",
        run_id,
        iteration,
        receipt.node_id.as_str()
    ))
}

fn node_receipt_evidence_summary(receipt: &GraphNodeExecutionReceipt) -> String {
    format!(
        "node {} executor {} status {:?} diagnostics={}",
        receipt.node_id.as_str(),
        receipt.executor.as_str(),
        receipt.status,
        receipt.diagnostics.len()
    )
}

fn node_receipt_evidence_score_basis_points(receipt: &GraphNodeExecutionReceipt) -> u16 {
    if receipt.diagnostics.is_empty() {
        8_000
    } else {
        9_000
    }
}
