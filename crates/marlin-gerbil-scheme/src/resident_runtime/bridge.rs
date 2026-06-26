//! Procedure bridges for resident `Gerbil Scheme` strategy execution.

use std::{
    collections::BTreeMap,
    io,
    process::{Command, Output},
};

use crate::{GerbilCommandProfile, scheme_types::GerbilSchemeValue};

use super::{
    GerbilResidentRuntimeSessionId, GerbilResidentStrategyEventKind,
    GerbilResidentStrategyExecutionRequest, GerbilResidentStrategyExecutionResponse,
    GerbilResidentStrategyExecutionStatus,
};

/// Executes an admitted resident strategy request.
pub trait GerbilResidentStrategyExecutor {
    fn execute(
        &mut self,
        request: &GerbilResidentStrategyExecutionRequest,
    ) -> io::Result<GerbilResidentStrategyExecutionResponse>;
}

impl<F> GerbilResidentStrategyExecutor for F
where
    F: FnMut(&GerbilResidentStrategyExecutionRequest) -> GerbilResidentStrategyExecutionResponse,
{
    fn execute(
        &mut self,
        request: &GerbilResidentStrategyExecutionRequest,
    ) -> io::Result<GerbilResidentStrategyExecutionResponse> {
        Ok(self(request))
    }
}

/// Slow `gxi` backed smoke bridge for the real `Gerbil Scheme` strategy procedure.
///
/// This exists to validate the typed procedure contract and receipt projection
/// against a real Gerbil runtime. It is not a runtime policy executor: hot
/// `.ss` policy integration must use native AOT or another already-resident
/// native transport so Rust policy dispatch stays in the millisecond to
/// sub-millisecond budget.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GerbilResidentStrategyGxiSmokeBridge {
    pub command_profile: GerbilCommandProfile,
    pub procedure_module: String,
    pub procedure_name: String,
}

impl GerbilResidentStrategyGxiSmokeBridge {
    pub fn new(
        command_profile: GerbilCommandProfile,
        procedure_module: impl Into<String>,
        procedure_name: impl Into<String>,
    ) -> Self {
        Self {
            command_profile,
            procedure_module: procedure_module.into(),
            procedure_name: procedure_name.into(),
        }
    }

    pub fn marlin_deck_runtime(command_profile: GerbilCommandProfile) -> Self {
        Self::new(
            command_profile,
            ":marlin/deck-runtime-strategy",
            "marlin-deck-runtime-resident-strategy-execute",
        )
    }

    fn command_output(
        &self,
        request: &GerbilResidentStrategyExecutionRequest,
    ) -> io::Result<Output> {
        let request_value = resident_strategy_request_value(request);
        let expression = format!(
            "(begin (import {}) (write ({} (quote {}))) (newline))",
            self.procedure_module,
            self.procedure_name,
            scheme_value_to_datum(&request_value)
        );
        let mut command = Command::new(&self.command_profile.program);
        command.args(&self.command_profile.args);
        command.arg("-e").arg(expression);

        if let Some(current_dir) = &self.command_profile.current_dir {
            command.current_dir(current_dir);
        }
        command.envs(&self.command_profile.env);

        command.output()
    }
}

impl GerbilResidentStrategyExecutor for GerbilResidentStrategyGxiSmokeBridge {
    fn execute(
        &mut self,
        request: &GerbilResidentStrategyExecutionRequest,
    ) -> io::Result<GerbilResidentStrategyExecutionResponse> {
        let output = self.command_output(request)?;
        if !output.status.success() {
            return Ok(GerbilResidentStrategyExecutionResponse::runtime_error(
                bridge_command_failure_reason(&output),
            ));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let Some(line) = stdout.lines().rev().find(|line| !line.trim().is_empty()) else {
            return Ok(GerbilResidentStrategyExecutionResponse::runtime_error(
                "resident Gerbil strategy procedure returned no output",
            ));
        };

        match parse_scheme_value_datum(line.trim())
            .and_then(GerbilResidentStrategyExecutionResponse::from_scheme_bridge_value)
        {
            Ok(response) => Ok(response),
            Err(error) => Ok(GerbilResidentStrategyExecutionResponse::runtime_error(
                error,
            )),
        }
    }
}

impl GerbilResidentStrategyExecutionResponse {
    pub fn from_scheme_bridge_value(value: GerbilSchemeValue) -> Result<Self, String> {
        let status = value
            .get("status")
            .and_then(GerbilSchemeValue::as_text)
            .ok_or_else(|| "resident strategy response is missing text status".to_string())?;
        let status = strategy_status_from_bridge_text(status)?;
        let payload = optional_bridge_value(value.get("payload").cloned());
        let reason = value
            .get("reason")
            .and_then(GerbilSchemeValue::as_text)
            .map(ToOwned::to_owned);
        let derived_session_id = value
            .get("derived_session_id")
            .and_then(GerbilSchemeValue::as_text)
            .filter(|value| !value.is_empty())
            .map(GerbilResidentRuntimeSessionId::new);

        Ok(Self {
            status,
            payload,
            reason,
            derived_session_id,
        })
    }
}

fn resident_strategy_request_value(
    request: &GerbilResidentStrategyExecutionRequest,
) -> GerbilSchemeValue {
    GerbilSchemeValue::record([
        (
            "request_id",
            GerbilSchemeValue::text(request.strategy_request.request_id.as_str()),
        ),
        (
            "lane_id",
            GerbilSchemeValue::text(request.strategy_request.lane_id.as_str()),
        ),
        (
            "event_kind",
            GerbilSchemeValue::text(strategy_event_kind_text(
                &request.strategy_request.event_kind,
            )),
        ),
        (
            "session_id",
            request
                .strategy_request
                .session_id
                .as_ref()
                .map(|session_id| GerbilSchemeValue::text(session_id.as_str()))
                .unwrap_or_else(GerbilSchemeValue::null),
        ),
        (
            "policy_epoch",
            request
                .strategy_request
                .policy_epoch
                .map(|policy_epoch| GerbilSchemeValue::integer(policy_epoch as i64))
                .unwrap_or_else(GerbilSchemeValue::null),
        ),
        ("payload", request.payload.clone()),
    ])
}

fn strategy_event_kind_text(event_kind: &GerbilResidentStrategyEventKind) -> &'static str {
    match event_kind {
        GerbilResidentStrategyEventKind::DynamicReplan => "dynamic-replan",
        GerbilResidentStrategyEventKind::PolicyChange => "policy-change",
    }
}

fn strategy_status_from_bridge_text(
    status: &str,
) -> Result<GerbilResidentStrategyExecutionStatus, String> {
    match status {
        "executed" => Ok(GerbilResidentStrategyExecutionStatus::Executed),
        "denied" => Ok(GerbilResidentStrategyExecutionStatus::Denied),
        "deferred" => Ok(GerbilResidentStrategyExecutionStatus::Deferred),
        "rejected-by-contract" => Ok(GerbilResidentStrategyExecutionStatus::RejectedByContract),
        "runtime-error" => Ok(GerbilResidentStrategyExecutionStatus::RuntimeError),
        "admission-rejected" => Ok(GerbilResidentStrategyExecutionStatus::AdmissionRejected),
        _ => Err(format!(
            "unknown resident strategy response status: {status}"
        )),
    }
}

fn optional_bridge_value(value: Option<GerbilSchemeValue>) -> Option<GerbilSchemeValue> {
    value.filter(|value| !value.is_null())
}

fn bridge_command_failure_reason(output: &Output) -> String {
    format!(
        "resident Gerbil strategy procedure failed status={:?} stdout={} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).trim(),
        String::from_utf8_lossy(&output.stderr).trim()
    )
}

fn scheme_value_to_datum(value: &GerbilSchemeValue) -> String {
    match value {
        GerbilSchemeValue::Null => "null".to_string(),
        GerbilSchemeValue::Boolean(value) => {
            if *value {
                "#t".to_string()
            } else {
                "#f".to_string()
            }
        }
        GerbilSchemeValue::Integer(value) => value.to_string(),
        GerbilSchemeValue::Float(value) => value.to_string(),
        GerbilSchemeValue::Text(value) => scheme_string(value),
        GerbilSchemeValue::Vector(values) => {
            let values = values
                .iter()
                .map(scheme_value_to_datum)
                .collect::<Vec<_>>()
                .join(" ");
            format!("(vector {values})")
        }
        GerbilSchemeValue::Record(fields) => {
            let fields = fields
                .iter()
                .map(|(key, value)| {
                    format!("({} {})", scheme_string(key), scheme_value_to_datum(value))
                })
                .collect::<Vec<_>>()
                .join(" ");
            format!("(record {fields})")
        }
    }
}

fn scheme_string(value: &str) -> String {
    let mut encoded = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => encoded.push_str("\\\\"),
            '"' => encoded.push_str("\\\""),
            '\n' => encoded.push_str("\\n"),
            '\r' => encoded.push_str("\\r"),
            '\t' => encoded.push_str("\\t"),
            _ => encoded.push(ch),
        }
    }
    encoded.push('"');
    encoded
}

fn parse_scheme_value_datum(input: &str) -> Result<GerbilSchemeValue, String> {
    let mut parser = SchemeDatumParser::new(input);
    let value = parser.parse_value()?;
    parser.skip_whitespace();
    if parser.is_finished() {
        Ok(value)
    } else {
        Err("resident strategy response has trailing datum content".to_string())
    }
}

struct SchemeDatumParser<'a> {
    input: &'a str,
    offset: usize,
}

impl<'a> SchemeDatumParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, offset: 0 }
    }

    fn parse_value(&mut self) -> Result<GerbilSchemeValue, String> {
        self.skip_whitespace();
        match self.peek_char() {
            Some('(') => self.parse_list(),
            Some('"') => self.parse_text().map(GerbilSchemeValue::text),
            Some('#') => self.parse_boolean(),
            Some(_) => self.parse_atom(),
            None => Err("resident strategy response ended before a datum".to_string()),
        }
    }

    fn parse_list(&mut self) -> Result<GerbilSchemeValue, String> {
        self.expect_char('(')?;
        let mut values = Vec::new();
        loop {
            self.skip_whitespace();
            if self.consume_char(')') {
                break;
            }
            values.push(self.parse_value()?);
        }

        let Some(GerbilSchemeValue::Text(kind)) = values.first() else {
            return Ok(GerbilSchemeValue::vector(values));
        };
        match kind.as_str() {
            "record" => record_from_datum_entries(values.into_iter().skip(1)),
            "vector" => Ok(GerbilSchemeValue::vector(values.into_iter().skip(1))),
            _ => Ok(GerbilSchemeValue::vector(values)),
        }
    }

    fn parse_text(&mut self) -> Result<String, String> {
        self.expect_char('"')?;
        let mut value = String::new();
        loop {
            let Some(ch) = self.next_char() else {
                return Err("resident strategy response has unterminated string".to_string());
            };
            match ch {
                '"' => return Ok(value),
                '\\' => {
                    let Some(escaped) = self.next_char() else {
                        return Err(
                            "resident strategy response has dangling string escape".to_string()
                        );
                    };
                    match escaped {
                        '"' => value.push('"'),
                        '\\' => value.push('\\'),
                        'n' => value.push('\n'),
                        'r' => value.push('\r'),
                        't' => value.push('\t'),
                        other => value.push(other),
                    }
                }
                other => value.push(other),
            }
        }
    }

    fn parse_boolean(&mut self) -> Result<GerbilSchemeValue, String> {
        if self.consume_str("#t") {
            Ok(GerbilSchemeValue::boolean(true))
        } else if self.consume_str("#f") {
            Ok(GerbilSchemeValue::boolean(false))
        } else {
            Err("resident strategy response has invalid boolean datum".to_string())
        }
    }

    fn parse_atom(&mut self) -> Result<GerbilSchemeValue, String> {
        let atom = self.take_atom();
        if atom.is_empty() {
            return Err("resident strategy response has empty atom".to_string());
        }
        if atom == "null" {
            return Ok(GerbilSchemeValue::null());
        }
        if let Ok(value) = atom.parse::<i64>() {
            return Ok(GerbilSchemeValue::integer(value));
        }
        if let Ok(value) = atom.parse::<f64>() {
            return Ok(GerbilSchemeValue::float(value));
        }
        Ok(GerbilSchemeValue::text(atom))
    }

    fn take_atom(&mut self) -> String {
        let start = self.offset;
        while let Some(ch) = self.peek_char() {
            if ch.is_whitespace() || ch == '(' || ch == ')' {
                break;
            }
            self.next_char();
        }
        self.input[start..self.offset].to_string()
    }

    fn skip_whitespace(&mut self) {
        while self.peek_char().is_some_and(char::is_whitespace) {
            self.next_char();
        }
    }

    fn expect_char(&mut self, expected: char) -> Result<(), String> {
        if self.consume_char(expected) {
            Ok(())
        } else {
            Err(format!(
                "resident strategy response expected `{expected}` at byte {}",
                self.offset
            ))
        }
    }

    fn consume_char(&mut self, expected: char) -> bool {
        if self.peek_char() == Some(expected) {
            self.next_char();
            true
        } else {
            false
        }
    }

    fn consume_str(&mut self, expected: &str) -> bool {
        if self.input[self.offset..].starts_with(expected) {
            self.offset += expected.len();
            true
        } else {
            false
        }
    }

    fn peek_char(&self) -> Option<char> {
        self.input[self.offset..].chars().next()
    }

    fn next_char(&mut self) -> Option<char> {
        let ch = self.peek_char()?;
        self.offset += ch.len_utf8();
        Some(ch)
    }

    fn is_finished(&self) -> bool {
        self.offset == self.input.len()
    }
}

fn record_from_datum_entries(
    entries: impl Iterator<Item = GerbilSchemeValue>,
) -> Result<GerbilSchemeValue, String> {
    let mut fields = BTreeMap::new();
    for entry in entries {
        let GerbilSchemeValue::Vector(mut values) = entry else {
            return Err("resident strategy record entry is not a field pair".to_string());
        };
        if values.len() != 2 {
            return Err("resident strategy record field must contain key and value".to_string());
        }
        let value = values.pop().expect("field value present");
        let key = values.pop().expect("field key present");
        let Some(key) = key.as_text().map(ToOwned::to_owned) else {
            return Err("resident strategy record field key is not text".to_string());
        };
        fields.insert(key, value);
    }
    Ok(GerbilSchemeValue::Record(fields))
}
