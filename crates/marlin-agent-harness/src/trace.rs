//! Lightweight `tracing` capture utilities for harness scenario tests.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use marlin_agent_protocol::{AgentSpanName, AgentTraceSpanRecord};
use tracing::field::{Field, Visit};
use tracing::span::{Attributes, Id, Record};
use tracing::subscriber::Interest;
use tracing::{Event, Metadata, Subscriber};

/// Minimal in-memory `tracing` subscriber used by harness tests.
#[derive(Clone, Debug)]
pub struct TraceRecorder {
    inner: Arc<TraceRecorderInner>,
}

#[derive(Debug)]
struct TraceRecorderInner {
    spans: Mutex<Vec<AgentTraceSpanRecord>>,
    span_indices: Mutex<BTreeMap<u64, usize>>,
    next_id: AtomicU64,
}

impl Default for TraceRecorder {
    fn default() -> Self {
        Self::new()
    }
}

impl TraceRecorder {
    /// Creates an empty trace recorder.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(TraceRecorderInner {
                spans: Mutex::new(Vec::new()),
                span_indices: Mutex::new(BTreeMap::new()),
                next_id: AtomicU64::new(1),
            }),
        }
    }

    /// Installs this recorder as the default subscriber for the current thread.
    ///
    /// Hold the returned guard for the full scope that creates spans. This is
    /// intentionally harness-local and does not install any global subscriber.
    pub fn install(&self) -> impl Drop {
        tracing::subscriber::set_default(self.clone())
    }

    /// Returns all span records captured so far.
    pub fn spans(&self) -> Vec<AgentTraceSpanRecord> {
        self.inner
            .spans
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Returns the captured span names in creation order.
    pub fn span_names(&self) -> Vec<AgentSpanName> {
        self.spans().into_iter().map(|span| span.name).collect()
    }

    /// Returns true when any captured span has the supplied name.
    pub fn contains_span(&self, name: &AgentSpanName) -> bool {
        self.inner
            .spans
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .any(|span| &span.name == name)
    }

    /// Counts captured spans with the supplied name.
    pub fn count_span(&self, name: &AgentSpanName) -> usize {
        self.inner
            .spans
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .filter(|span| &span.name == name)
            .count()
    }
}

impl Subscriber for TraceRecorder {
    fn register_callsite(&self, _metadata: &'static Metadata<'static>) -> Interest {
        Interest::always()
    }

    fn enabled(&self, _metadata: &Metadata<'_>) -> bool {
        true
    }

    fn new_span(&self, attributes: &Attributes<'_>) -> Id {
        let mut fields = BTreeMap::new();
        attributes.record(&mut TraceFieldRecorder {
            fields: &mut fields,
        });
        let span_id = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        let span_index = {
            let mut spans = self
                .inner
                .spans
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let span_index = spans.len();
            spans.push(AgentTraceSpanRecord {
                name: AgentSpanName::new(attributes.metadata().name()),
                fields,
            });
            span_index
        };
        self.inner
            .span_indices
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(span_id, span_index);

        Id::from_u64(span_id)
    }

    fn record(&self, span: &Id, values: &Record<'_>) {
        let span_id = span.clone().into_u64();
        let Some(span_index) = self
            .inner
            .span_indices
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&span_id)
            .copied()
        else {
            return;
        };

        let mut fields = BTreeMap::new();
        values.record(&mut TraceFieldRecorder {
            fields: &mut fields,
        });
        if fields.is_empty() {
            return;
        }

        if let Some(span_record) = self
            .inner
            .spans
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get_mut(span_index)
        {
            span_record.fields.extend(fields);
        }
    }

    fn record_follows_from(&self, _span: &Id, _follows: &Id) {}

    fn event(&self, _event: &Event<'_>) {}

    fn enter(&self, _span: &Id) {}

    fn exit(&self, _span: &Id) {}
}

struct TraceFieldRecorder<'a> {
    fields: &'a mut BTreeMap<String, String>,
}

impl TraceFieldRecorder<'_> {
    fn insert(&mut self, field: &Field, value: String) {
        self.fields.insert(field.name().to_owned(), value);
    }
}

impl Visit for TraceFieldRecorder<'_> {
    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        self.insert(field, format!("{value:?}"));
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.insert(field, value.to_owned());
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.insert(field, value.to_string());
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.insert(field, value.to_string());
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.insert(field, value.to_string());
    }
}
