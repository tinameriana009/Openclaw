use std::fs;
use std::path::Path;
use std::sync::Arc;

use runtime::{
    Citation, ConfidenceLevel, ConfidenceNote, EvidenceProvenance, ExecutionProfile, FinalAnswer,
    TraceLedger,
};
use telemetry::{MemoryTelemetrySink, SessionTracer, TelemetryEvent};

fn fixture_path(relative: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(relative)
}

#[test]
fn final_answer_matches_expected_fixture() {
    let answer = FinalAnswer {
        body: "Implemented the recursive trace summary and grounding formatter.".to_string(),
        citations: vec![
            Citation {
                label: "L1".to_string(),
                provenance: EvidenceProvenance::Local,
                title: "runtime/src/trace.rs".to_string(),
                locator: Some("summary counters".to_string()),
            },
            Citation {
                label: "L2".to_string(),
                provenance: EvidenceProvenance::Local,
                title: "runtime/src/ux.rs".to_string(),
                locator: Some("final answer renderer".to_string()),
            },
        ],
        confidence: Some(ConfidenceNote {
            level: ConfidenceLevel::High,
            summary: "Covered by unit and regression tests.".to_string(),
            gaps: vec!["Recursive orchestration is still scaffold-level in this repo.".to_string()],
        }),
        web: None,
        trace_id: Some("trace-fixture-001".to_string()),
    };

    let expected = fs::read_to_string(fixture_path("expected_final_answer.txt"))
        .expect("expected fixture should load");
    assert_eq!(answer.render_text(), expected.trim_end());
}

#[test]
fn trace_fixture_round_trips_and_emits_safe_telemetry() {
    let trace = TraceLedger::read_from_path(&fixture_path("trace_fixture.json"))
        .expect("trace fixture should parse");
    let sink = Arc::new(MemoryTelemetrySink::default());
    let tracer = SessionTracer::new("session-fixture", sink.clone());

    trace.emit_telemetry(&tracer);

    let events = sink.events();
    assert!(events.iter().any(|event| match event {
        TelemetryEvent::SessionTrace(record) => {
            record.name == "recursive_trace_summary"
                && record.attributes.get("retrieval_requests") == Some(&serde_json::json!(1))
                && record.attributes.get("web_evidence_items") == Some(&serde_json::json!(1))
        }
        _ => false,
    }));
    assert!(events.iter().all(|event| match event {
        TelemetryEvent::SessionTrace(record) => !record.attributes.contains_key("preview"),
        _ => true,
    }));
}

#[test]
fn execution_profiles_form_an_increasing_budget_ladder() {
    let fast = ExecutionProfile::Fast.resolve();
    let balanced = ExecutionProfile::Balanced.resolve();
    let deep = ExecutionProfile::Deep.resolve();
    let research = ExecutionProfile::Research.resolve();

    assert!(fast.rag.max_hits < balanced.rag.max_hits);
    assert!(balanced.rlm.max_depth < deep.rlm.max_depth);
    assert!(deep.rlm.max_runtime_ms < research.rlm.max_runtime_ms);
}
