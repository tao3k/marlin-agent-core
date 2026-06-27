//! Typed receipt projection for real LLM loop case runner diagnostics.

use serde::{Deserialize, Serialize};

/// Rust-owned receipt projected from the real LLM loop case CLI boundary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GerbilLoopCaseDriverRealLlmCaseReceipt {
    case_id: String,
    result: String,
    rounds_used: u64,
    terminal_status: String,
    iteration_count: u64,
    process_exit_status: i32,
    continuation_planner: Option<String>,
    failure_classification_receipt_present: bool,
    governance_receipt_present: bool,
    nono_sandbox_materialized: bool,
    human_audit_decision: bool,
}

impl GerbilLoopCaseDriverRealLlmCaseReceipt {
    #[must_use]
    pub fn case_id(&self) -> &str {
        &self.case_id
    }

    #[must_use]
    pub fn result(&self) -> &str {
        &self.result
    }

    #[must_use]
    pub const fn rounds_used(&self) -> u64 {
        self.rounds_used
    }

    #[must_use]
    pub fn terminal_status(&self) -> &str {
        &self.terminal_status
    }

    #[must_use]
    pub const fn iteration_count(&self) -> u64 {
        self.iteration_count
    }

    #[must_use]
    pub const fn process_exit_status(&self) -> i32 {
        self.process_exit_status
    }

    #[must_use]
    pub fn continuation_planner(&self) -> Option<&str> {
        self.continuation_planner.as_deref()
    }

    #[must_use]
    pub const fn failure_classification_receipt_present(&self) -> bool {
        self.failure_classification_receipt_present
    }

    #[must_use]
    pub const fn governance_receipt_present(&self) -> bool {
        self.governance_receipt_present
    }

    #[must_use]
    pub const fn nono_sandbox_materialized(&self) -> bool {
        self.nono_sandbox_materialized
    }

    #[must_use]
    pub const fn human_audit_decision(&self) -> bool {
        self.human_audit_decision
    }
}

/// Parse the real LLM loop case runner output into a Rust-owned receipt.
///
/// This parser is only for the CLI/trace boundary. The Scheme config-interface
/// policy projection remains Scheme types -> Rust types; this does not create a
/// Scheme-internal text protocol.
pub fn parse_gerbil_loop_case_driver_real_llm_case_receipt(
    stdout: &str,
) -> Result<GerbilLoopCaseDriverRealLlmCaseReceipt, GerbilLoopCaseDriverRealLlmCaseReceiptError> {
    let trace = stdout_trace_object(stdout)?;
    let trace_root = serde_json::Value::Object(trace.clone());

    Ok(GerbilLoopCaseDriverRealLlmCaseReceipt {
        case_id: required_marker(stdout, "marlin-real-llm-case.case_id=")?,
        result: required_marker(stdout, "marlin-real-llm-case.result=")?,
        rounds_used: parse_marker(stdout, "marlin-real-llm-case.rounds_used=")?,
        terminal_status: required_trace_string_field(&trace, "terminal_status")?,
        iteration_count: required_trace_u64_field(&trace, "iteration_count")?,
        process_exit_status: parse_marker(stdout, "process-command.exit_status:")?,
        continuation_planner: optional_marker(stdout, "continuation_planner="),
        failure_classification_receipt_present: trace_contains_key(
            &trace_root,
            "failure_classification_receipt",
        ),
        governance_receipt_present: trace_contains_key(&trace_root, "governance_receipt"),
        nono_sandbox_materialized: trace_path_string_eq(&trace_root, &["backend"], "nono")
            || trace_path_string_eq(
                &trace_root,
                &["governance_receipt", "sandbox", "backend"],
                "nono",
            ),
        human_audit_decision: trace_path_string_eq(&trace_root, &["decision"], "human-audit")
            || trace_path_string_eq(
                &trace_root,
                &["governance_receipt", "verifier", "decision"],
                "human-audit",
            ),
    })
}

fn trace_contains_key(value: &serde_json::Value, key: &str) -> bool {
    match value {
        serde_json::Value::Object(map) => {
            map.contains_key(key) || map.values().any(|value| trace_contains_key(value, key))
        }
        serde_json::Value::Array(values) => {
            values.iter().any(|value| trace_contains_key(value, key))
        }
        _ => false,
    }
}

fn trace_path_string_eq(value: &serde_json::Value, path: &[&str], expected: &str) -> bool {
    let Some((first, rest)) = path.split_first() else {
        return value.as_str() == Some(expected);
    };
    let Some(next) = value.get(*first) else {
        return false;
    };
    trace_path_string_eq(next, rest, expected)
}

/// Parse error for real LLM loop case runner receipts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GerbilLoopCaseDriverRealLlmCaseReceiptError {
    MissingMarker {
        marker: &'static str,
    },
    MissingField {
        field: &'static str,
    },
    InvalidNumber {
        field: &'static str,
        value: String,
        message: String,
    },
    InvalidTraceJson {
        message: String,
    },
}

impl std::fmt::Display for GerbilLoopCaseDriverRealLlmCaseReceiptError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingMarker { marker } => write!(formatter, "missing marker {marker}"),
            Self::MissingField { field } => write!(formatter, "missing field {field}"),
            Self::InvalidNumber {
                field,
                value,
                message,
            } => write!(
                formatter,
                "invalid numeric field {field} value {value:?}: {message}"
            ),
            Self::InvalidTraceJson { message } => {
                write!(formatter, "invalid real LLM trace JSON: {message}")
            }
        }
    }
}

impl std::error::Error for GerbilLoopCaseDriverRealLlmCaseReceiptError {}

fn parse_marker<T>(
    stdout: &str,
    marker: &'static str,
) -> Result<T, GerbilLoopCaseDriverRealLlmCaseReceiptError>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let value = required_marker(stdout, marker)?;
    value.parse::<T>().map_err(
        |error| GerbilLoopCaseDriverRealLlmCaseReceiptError::InvalidNumber {
            field: marker,
            value,
            message: error.to_string(),
        },
    )
}

fn required_marker(
    stdout: &str,
    marker: &'static str,
) -> Result<String, GerbilLoopCaseDriverRealLlmCaseReceiptError> {
    optional_marker(stdout, marker)
        .ok_or(GerbilLoopCaseDriverRealLlmCaseReceiptError::MissingMarker { marker })
}

fn optional_marker(stdout: &str, marker: &'static str) -> Option<String> {
    let start = stdout.find(marker)? + marker.len();
    let rest = &stdout[start..];
    let end = rest.find(['\\', '"', '\n', '\r']).unwrap_or(rest.len());
    Some(rest[..end].to_owned())
}

fn stdout_trace_object(
    stdout: &str,
) -> Result<serde_json::Map<String, serde_json::Value>, GerbilLoopCaseDriverRealLlmCaseReceiptError>
{
    let start = stdout.find('{').ok_or_else(|| {
        GerbilLoopCaseDriverRealLlmCaseReceiptError::InvalidTraceJson {
            message: "missing trace object".to_owned(),
        }
    })?;
    let mut values = serde_json::Deserializer::from_str(&stdout[start..]).into_iter();
    let value: serde_json::Value = values
        .next()
        .ok_or_else(
            || GerbilLoopCaseDriverRealLlmCaseReceiptError::InvalidTraceJson {
                message: "missing trace object".to_owned(),
            },
        )?
        .map_err(
            |error| GerbilLoopCaseDriverRealLlmCaseReceiptError::InvalidTraceJson {
                message: error.to_string(),
            },
        )?;

    value.as_object().cloned().ok_or_else(|| {
        GerbilLoopCaseDriverRealLlmCaseReceiptError::InvalidTraceJson {
            message: "trace root is not an object".to_owned(),
        }
    })
}

fn required_trace_string_field(
    trace: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<String, GerbilLoopCaseDriverRealLlmCaseReceiptError> {
    trace
        .get(field)
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned)
        .ok_or(GerbilLoopCaseDriverRealLlmCaseReceiptError::MissingField { field })
}

fn required_trace_u64_field(
    trace: &serde_json::Map<String, serde_json::Value>,
    field: &'static str,
) -> Result<u64, GerbilLoopCaseDriverRealLlmCaseReceiptError> {
    trace
        .get(field)
        .and_then(serde_json::Value::as_u64)
        .ok_or(GerbilLoopCaseDriverRealLlmCaseReceiptError::MissingField { field })
}
