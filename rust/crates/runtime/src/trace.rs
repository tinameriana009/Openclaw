use std::collections::BTreeMap;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::Path;

use serde_json::{Map, Value};
use telemetry::SessionTracer;

use crate::json::{JsonError, JsonValue};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceLedger {
    pub trace_id: String,
    pub session_id: String,
    pub root_task_id: String,
    pub started_at_ms: u64,
    pub finished_at_ms: Option<u64>,
    pub final_status: TraceFinalStatus,
    pub events: Vec<TraceEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceFinalStatus {
    Running,
    Succeeded,
    Failed,
    Cancelled,
    BudgetExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceEvent {
    pub sequence: u32,
    pub event_type: TraceEventType,
    pub timestamp_ms: u64,
    pub data: BTreeMap<String, JsonValue>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TraceCounters {
    pub retrieval_requests: u32,
    pub retrieval_completions: u32,
    pub subqueries_started: u32,
    pub subqueries_completed: u32,
    pub web_escalations: u32,
    pub web_evidence_items: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceEventType {
    TaskStarted,
    RetrievalRequested,
    RetrievalCompleted,
    CorpusPeeked,
    CorpusSliced,
    SubqueryStarted,
    SubqueryCompleted,
    WebEscalationStarted,
    WebEvidenceAdded,
    AggregationCompleted,
    StopConditionReached,
    TaskFailed,
}

#[derive(Debug)]
pub enum TraceError {
    Io(std::io::Error),
    Json(JsonError),
    Format(String),
}

impl Display for TraceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "{error}"),
            Self::Json(error) => write!(f, "{error}"),
            Self::Format(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for TraceError {}

impl From<std::io::Error> for TraceError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<JsonError> for TraceError {
    fn from(value: JsonError) -> Self {
        Self::Json(value)
    }
}

impl TraceLedger {
    #[must_use]
    pub fn to_json_value(&self) -> JsonValue {
        JsonValue::Object(BTreeMap::from([
            (
                "traceId".to_string(),
                JsonValue::String(self.trace_id.clone()),
            ),
            (
                "sessionId".to_string(),
                JsonValue::String(self.session_id.clone()),
            ),
            (
                "rootTaskId".to_string(),
                JsonValue::String(self.root_task_id.clone()),
            ),
            (
                "startedAtMs".to_string(),
                JsonValue::Number(i64::try_from(self.started_at_ms).unwrap_or(i64::MAX)),
            ),
            (
                "finishedAtMs".to_string(),
                self.finished_at_ms
                    .map(|value| JsonValue::Number(i64::try_from(value).unwrap_or(i64::MAX)))
                    .unwrap_or(JsonValue::Null),
            ),
            (
                "finalStatus".to_string(),
                JsonValue::String(self.final_status.as_str().to_string()),
            ),
            (
                "events".to_string(),
                JsonValue::Array(self.events.iter().map(TraceEvent::to_json_value).collect()),
            ),
        ]))
    }

    pub fn from_json_value(value: &JsonValue) -> Result<Self, TraceError> {
        let object = value
            .as_object()
            .ok_or_else(|| TraceError::Format("trace ledger must be a JSON object".to_string()))?;
        Ok(Self {
            trace_id: expect_string(object, "traceId")?.to_string(),
            session_id: expect_string(object, "sessionId")?.to_string(),
            root_task_id: expect_string(object, "rootTaskId")?.to_string(),
            started_at_ms: expect_u64(object, "startedAtMs")?,
            finished_at_ms: optional_u64(object, "finishedAtMs")?,
            final_status: TraceFinalStatus::from_str(expect_string(object, "finalStatus")?)?,
            events: expect_events(object, "events")?,
        })
    }

    #[must_use]
    pub fn render_json(&self) -> String {
        self.to_json_value().render()
    }

    pub fn write_to_path(&self, path: &Path) -> Result<(), TraceError> {
        fs::write(path, self.render_json()).map_err(TraceError::Io)
    }

    pub fn read_from_path(path: &Path) -> Result<Self, TraceError> {
        let raw = fs::read_to_string(path)?;
        let value = JsonValue::parse(&raw)?;
        Self::from_json_value(&value)
    }

    #[must_use]
    pub fn summary_line(&self) -> String {
        format!(
            "trace={} session={} task={} status={} events={}",
            self.trace_id,
            self.session_id,
            self.root_task_id,
            self.final_status.as_str(),
            self.events.len()
        )
    }

    #[must_use]
    pub fn counters(&self) -> TraceCounters {
        let mut counters = TraceCounters::default();
        for event in &self.events {
            match event.event_type {
                TraceEventType::RetrievalRequested => counters.retrieval_requests += 1,
                TraceEventType::RetrievalCompleted => counters.retrieval_completions += 1,
                TraceEventType::SubqueryStarted => counters.subqueries_started += 1,
                TraceEventType::SubqueryCompleted => counters.subqueries_completed += 1,
                TraceEventType::WebEscalationStarted => counters.web_escalations += 1,
                TraceEventType::WebEvidenceAdded => counters.web_evidence_items += 1,
                _ => {}
            }
        }
        counters
    }

    pub fn emit_telemetry(&self, tracer: &SessionTracer) {
        let counters = self.counters();
        let mut summary = Map::new();
        summary.insert("trace_id".to_string(), Value::String(self.trace_id.clone()));
        summary.insert(
            "root_task_id".to_string(),
            Value::String(self.root_task_id.clone()),
        );
        summary.insert(
            "final_status".to_string(),
            Value::String(self.final_status.as_str().to_string()),
        );
        summary.insert(
            "event_count".to_string(),
            Value::from(self.events.len() as u64),
        );
        summary.insert(
            "retrieval_requests".to_string(),
            Value::from(counters.retrieval_requests),
        );
        summary.insert(
            "retrieval_completions".to_string(),
            Value::from(counters.retrieval_completions),
        );
        summary.insert(
            "subqueries_started".to_string(),
            Value::from(counters.subqueries_started),
        );
        summary.insert(
            "subqueries_completed".to_string(),
            Value::from(counters.subqueries_completed),
        );
        summary.insert(
            "web_escalations".to_string(),
            Value::from(counters.web_escalations),
        );
        summary.insert(
            "web_evidence_items".to_string(),
            Value::from(counters.web_evidence_items),
        );
        tracer.record("recursive_trace_summary", summary);

        for event in &self.events {
            let mut attributes = Map::new();
            attributes.insert("trace_id".to_string(), Value::String(self.trace_id.clone()));
            attributes.insert(
                "root_task_id".to_string(),
                Value::String(self.root_task_id.clone()),
            );
            attributes.insert(
                "event_type".to_string(),
                Value::String(event.event_type.as_str().to_string()),
            );
            attributes.insert("sequence".to_string(), Value::from(event.sequence));
            attributes.insert("timestamp_ms".to_string(), Value::from(event.timestamp_ms));
            if let Some(value) = event.data.get("hits").and_then(JsonValue::as_i64) {
                attributes.insert("hits".to_string(), Value::from(value));
            }
            if let Some(value) = event.data.get("stopReason").and_then(JsonValue::as_str) {
                attributes.insert("stop_reason".to_string(), Value::String(value.to_string()));
            }
            tracer.record("recursive_trace_event", attributes);
        }
    }
}

impl TraceFinalStatus {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::BudgetExceeded => "budget_exceeded",
        }
    }

    pub fn from_str(value: &str) -> Result<Self, TraceError> {
        match value {
            "running" => Ok(Self::Running),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            "cancelled" => Ok(Self::Cancelled),
            "budget_exceeded" => Ok(Self::BudgetExceeded),
            other => Err(TraceError::Format(format!(
                "unsupported trace final status {other}"
            ))),
        }
    }
}

impl TraceEvent {
    #[must_use]
    pub fn new(
        sequence: u32,
        event_type: TraceEventType,
        timestamp_ms: u64,
        data: BTreeMap<String, JsonValue>,
    ) -> Self {
        Self {
            sequence,
            event_type,
            timestamp_ms,
            data,
        }
    }

    #[must_use]
    pub fn to_json_value(&self) -> JsonValue {
        JsonValue::Object(BTreeMap::from([
            (
                "sequence".to_string(),
                JsonValue::Number(i64::from(self.sequence)),
            ),
            (
                "eventType".to_string(),
                JsonValue::String(self.event_type.as_str().to_string()),
            ),
            (
                "timestampMs".to_string(),
                JsonValue::Number(i64::try_from(self.timestamp_ms).unwrap_or(i64::MAX)),
            ),
            ("data".to_string(), JsonValue::Object(self.data.clone())),
        ]))
    }

    pub fn from_json_value(value: &JsonValue) -> Result<Self, TraceError> {
        let object = value
            .as_object()
            .ok_or_else(|| TraceError::Format("trace event must be a JSON object".to_string()))?;
        Ok(Self {
            sequence: u32::try_from(expect_u64(object, "sequence")?)
                .map_err(|_| TraceError::Format("sequence is out of range for u32".to_string()))?,
            event_type: TraceEventType::from_str(expect_string(object, "eventType")?)?,
            timestamp_ms: expect_u64(object, "timestampMs")?,
            data: expect_object(object, "data")?.clone(),
        })
    }
}

impl TraceEventType {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::TaskStarted => "task_started",
            Self::RetrievalRequested => "retrieval_requested",
            Self::RetrievalCompleted => "retrieval_completed",
            Self::CorpusPeeked => "corpus_peeked",
            Self::CorpusSliced => "corpus_sliced",
            Self::SubqueryStarted => "subquery_started",
            Self::SubqueryCompleted => "subquery_completed",
            Self::WebEscalationStarted => "web_escalation_started",
            Self::WebEvidenceAdded => "web_evidence_added",
            Self::AggregationCompleted => "aggregation_completed",
            Self::StopConditionReached => "stop_condition_reached",
            Self::TaskFailed => "task_failed",
        }
    }

    pub fn from_str(value: &str) -> Result<Self, TraceError> {
        match value {
            "task_started" => Ok(Self::TaskStarted),
            "retrieval_requested" => Ok(Self::RetrievalRequested),
            "retrieval_completed" => Ok(Self::RetrievalCompleted),
            "corpus_peeked" => Ok(Self::CorpusPeeked),
            "corpus_sliced" => Ok(Self::CorpusSliced),
            "subquery_started" => Ok(Self::SubqueryStarted),
            "subquery_completed" => Ok(Self::SubqueryCompleted),
            "web_escalation_started" => Ok(Self::WebEscalationStarted),
            "web_evidence_added" => Ok(Self::WebEvidenceAdded),
            "aggregation_completed" => Ok(Self::AggregationCompleted),
            "stop_condition_reached" => Ok(Self::StopConditionReached),
            "task_failed" => Ok(Self::TaskFailed),
            other => Err(TraceError::Format(format!(
                "unsupported trace event type {other}"
            ))),
        }
    }
}

fn expect_string<'a>(
    object: &'a BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<&'a str, TraceError> {
    object
        .get(key)
        .and_then(JsonValue::as_str)
        .ok_or_else(|| TraceError::Format(format!("missing string field {key}")))
}

fn expect_u64(object: &BTreeMap<String, JsonValue>, key: &str) -> Result<u64, TraceError> {
    let value = object
        .get(key)
        .and_then(JsonValue::as_i64)
        .ok_or_else(|| TraceError::Format(format!("missing numeric field {key}")))?;
    u64::try_from(value)
        .map_err(|_| TraceError::Format(format!("numeric field {key} is out of range")))
}

fn optional_u64(
    object: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Option<u64>, TraceError> {
    match object.get(key) {
        Some(JsonValue::Null) | None => Ok(None),
        Some(JsonValue::Number(value)) => u64::try_from(*value)
            .map(Some)
            .map_err(|_| TraceError::Format(format!("numeric field {key} is out of range"))),
        Some(_) => Err(TraceError::Format(format!(
            "field {key} must be a number or null"
        ))),
    }
}

fn expect_object<'a>(
    object: &'a BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<&'a BTreeMap<String, JsonValue>, TraceError> {
    object
        .get(key)
        .and_then(JsonValue::as_object)
        .ok_or_else(|| TraceError::Format(format!("missing object field {key}")))
}

fn expect_events(
    object: &BTreeMap<String, JsonValue>,
    key: &str,
) -> Result<Vec<TraceEvent>, TraceError> {
    let values = object
        .get(key)
        .and_then(JsonValue::as_array)
        .ok_or_else(|| TraceError::Format(format!("missing array field {key}")))?;
    values.iter().map(TraceEvent::from_json_value).collect()
}

#[cfg(test)]
mod tests {
    use super::{TraceEvent, TraceEventType, TraceFinalStatus, TraceLedger};
    use crate::json::JsonValue;
    use std::collections::BTreeMap;
    use std::fs;
    use std::sync::Arc;
    use std::time::{SystemTime, UNIX_EPOCH};
    use telemetry::{MemoryTelemetrySink, SessionTracer, TelemetryEvent};

    fn temp_trace_path() -> std::path::PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("trace-ledger-{nanos}.json"))
    }

    fn sample_trace() -> TraceLedger {
        TraceLedger {
            trace_id: "trace-1".to_string(),
            session_id: "session-1".to_string(),
            root_task_id: "task-1".to_string(),
            started_at_ms: 1_700_000_000_000,
            finished_at_ms: Some(1_700_000_000_111),
            final_status: TraceFinalStatus::Succeeded,
            events: vec![
                TraceEvent::new(
                    1,
                    TraceEventType::TaskStarted,
                    1_700_000_000_000,
                    BTreeMap::from([(
                        "goal".to_string(),
                        JsonValue::String("analyze repository".to_string()),
                    )]),
                ),
                TraceEvent::new(
                    2,
                    TraceEventType::RetrievalCompleted,
                    1_700_000_000_050,
                    BTreeMap::from([("hits".to_string(), JsonValue::Number(7))]),
                ),
                TraceEvent::new(
                    3,
                    TraceEventType::SubqueryStarted,
                    1_700_000_000_060,
                    BTreeMap::new(),
                ),
                TraceEvent::new(
                    4,
                    TraceEventType::WebEscalationStarted,
                    1_700_000_000_070,
                    BTreeMap::new(),
                ),
            ],
        }
    }

    #[test]
    fn trace_round_trips_through_json_value() {
        let trace = sample_trace();
        let parsed = TraceLedger::from_json_value(&trace.to_json_value())
            .expect("trace should parse after round trip");
        assert_eq!(parsed, trace);
    }

    #[test]
    fn trace_writes_and_reads_from_file() {
        let path = temp_trace_path();
        let trace = sample_trace();
        trace.write_to_path(&path).expect("trace should write");
        let restored = TraceLedger::read_from_path(&path).expect("trace should read back");
        assert_eq!(restored, trace);
        fs::remove_file(path).expect("temp trace file should be removable");
    }

    #[test]
    fn summary_line_contains_core_identifiers() {
        let trace = sample_trace();
        let summary = trace.summary_line();
        assert!(summary.contains("trace=trace-1"));
        assert!(summary.contains("session=session-1"));
        assert!(summary.contains("task=task-1"));
        assert!(summary.contains("status=succeeded"));
        assert!(summary.contains("events=4"));
    }

    #[test]
    fn rejects_unknown_event_type() {
        let bad = JsonValue::parse(
            r#"{
              "traceId":"trace-1",
              "sessionId":"session-1",
              "rootTaskId":"task-1",
              "startedAtMs":1,
              "finishedAtMs":null,
              "finalStatus":"running",
              "events":[{
                "sequence":1,
                "eventType":"mystery_event",
                "timestampMs":1,
                "data":{}
              }]
            }"#,
        )
        .expect("json should parse");

        let error = TraceLedger::from_json_value(&bad).expect_err("unknown event should fail");
        assert!(error
            .to_string()
            .contains("unsupported trace event type mystery_event"));
    }

    #[test]
    fn counters_summarize_recursive_activity() {
        let counters = sample_trace().counters();
        assert_eq!(counters.retrieval_completions, 1);
        assert_eq!(counters.subqueries_started, 1);
        assert_eq!(counters.web_escalations, 1);
    }

    #[test]
    fn emit_telemetry_records_summary_and_per_event_entries() {
        let sink = Arc::new(MemoryTelemetrySink::default());
        let tracer = SessionTracer::new("session-1", sink.clone());
        let trace = sample_trace();

        trace.emit_telemetry(&tracer);

        let events = sink.events();
        let trace_records = events
            .into_iter()
            .filter_map(|event| match event {
                TelemetryEvent::SessionTrace(record) => Some(record),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert_eq!(trace_records.len(), trace.events.len() + 1);
        assert_eq!(trace_records[0].name, "recursive_trace_summary");
        assert_eq!(
            trace_records[0].attributes.get("web_escalations"),
            Some(&serde_json::Value::from(1))
        );
        assert!(trace_records.iter().any(|record| {
            record.name == "recursive_trace_event"
                && record.attributes.get("event_type")
                    == Some(&serde_json::Value::String(
                        "retrieval_completed".to_string(),
                    ))
                && record.attributes.get("hits") == Some(&serde_json::Value::from(7))
        }));
    }
}
