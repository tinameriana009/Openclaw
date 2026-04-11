use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use runtime::ContentBlock;
use runtime::Session;
use serde_json::Value as JsonValue;

static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[test]
fn resumed_binary_accepts_slash_commands_with_arguments() {
    // given
    let temp_dir = unique_temp_dir("resume-slash-commands");
    fs::create_dir_all(&temp_dir).expect("temp dir should exist");

    let session_path = temp_dir.join("session.jsonl");
    let export_path = temp_dir.join("notes.txt");

    let mut session = Session::new();
    session
        .push_user_text("ship the slash command harness")
        .expect("session write should succeed");
    session
        .save_to_path(&session_path)
        .expect("session should persist");

    // when
    let output = run_claw(
        &temp_dir,
        &[
            "--resume",
            session_path.to_str().expect("utf8 path"),
            "/export",
            export_path.to_str().expect("utf8 path"),
            "/clear",
            "--confirm",
        ],
    );

    // then
    assert!(
        output.status.success(),
        "stdout:\n{}\n\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Export"));
    assert!(stdout.contains("wrote transcript"));
    assert!(stdout.contains(export_path.to_str().expect("utf8 path")));
    assert!(stdout.contains("Session cleared"));
    assert!(stdout.contains("Mode             resumed session reset"));
    assert!(stdout.contains("Previous session"));
    assert!(stdout.contains("Resume previous  claw --resume"));
    assert!(stdout.contains("Backup           "));
    assert!(stdout.contains("Session file     "));

    let export = fs::read_to_string(&export_path).expect("export file should exist");
    assert!(export.contains("# Conversation Export"));
    assert!(export.contains("ship the slash command harness"));

    let restored = Session::load_from_path(&session_path).expect("cleared session should load");
    assert!(restored.messages.is_empty());

    let backup_path = stdout
        .lines()
        .find_map(|line| line.strip_prefix("  Backup           "))
        .map(PathBuf::from)
        .expect("clear output should include backup path");
    let backup = Session::load_from_path(&backup_path).expect("backup session should load");
    assert_eq!(backup.messages.len(), 1);
    assert!(matches!(
        backup.messages[0].blocks.first(),
        Some(ContentBlock::Text { text }) if text == "ship the slash command harness"
    ));
}

#[test]
fn status_command_applies_cli_flags_end_to_end() {
    // given
    let temp_dir = unique_temp_dir("status-command-flags");
    fs::create_dir_all(&temp_dir).expect("temp dir should exist");

    // when
    let output = run_claw(
        &temp_dir,
        &[
            "--model",
            "sonnet",
            "--permission-mode",
            "read-only",
            "status",
        ],
    );

    // then
    assert!(
        output.status.success(),
        "stdout:\n{}\n\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Status"));
    assert!(stdout.contains("Model            claude-sonnet-4-6"));
    assert!(stdout.contains("Permission mode  read-only"));
}

#[test]
fn resumed_config_command_loads_settings_files_end_to_end() {
    // given
    let temp_dir = unique_temp_dir("resume-config");
    let project_dir = temp_dir.join("project");
    let config_home = temp_dir.join("home").join(".claw");
    fs::create_dir_all(project_dir.join(".claw")).expect("project config dir should exist");
    fs::create_dir_all(&config_home).expect("config home should exist");

    let session_path = project_dir.join("session.jsonl");
    Session::new()
        .with_persistence_path(&session_path)
        .save_to_path(&session_path)
        .expect("session should persist");

    fs::write(config_home.join("settings.json"), r#"{"model":"haiku"}"#)
        .expect("user config should write");
    fs::write(
        project_dir.join(".claw").join("settings.local.json"),
        r#"{"model":"opus"}"#,
    )
    .expect("local config should write");

    // when
    let output = run_claw_with_env(
        &project_dir,
        &[
            "--resume",
            session_path.to_str().expect("utf8 path"),
            "/config",
            "model",
        ],
        &[("CLAW_CONFIG_HOME", config_home.to_str().expect("utf8 path"))],
    );

    // then
    assert!(
        output.status.success(),
        "stdout:\n{}\n\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Config"));
    assert!(stdout.contains("Loaded files      2"));
    assert!(stdout.contains(
        config_home
            .join("settings.json")
            .to_str()
            .expect("utf8 path")
    ));
    assert!(stdout.contains(
        project_dir
            .join(".claw")
            .join("settings.local.json")
            .to_str()
            .expect("utf8 path")
    ));
    assert!(stdout.contains("Merged section: model"));
    assert!(stdout.contains("opus"));
}

#[test]
fn resumed_trace_summary_reads_workspace_trace_artifacts() {
    let temp_dir = unique_temp_dir("resume-trace-summary");
    fs::create_dir_all(temp_dir.join(".claw").join("trace")).expect("trace dir should exist");

    let sessions_dir = temp_dir.join(".claw").join("sessions");
    fs::create_dir_all(&sessions_dir).expect("sessions dir should exist");
    let session_path = sessions_dir.join("session.jsonl");
    Session::new()
        .with_persistence_path(&session_path)
        .save_to_path(&session_path)
        .expect("session should persist");

    let trace_path = temp_dir.join(".claw").join("trace").join("trace.json");
    fs::write(
        &trace_path,
        r#"{
          "traceId":"trace-approval",
          "sessionId":"session-1",
          "rootTaskId":"task-1",
          "startedAtMs":1,
          "finishedAtMs":2,
          "finalStatus":"succeeded",
          "events":[{
            "sequence":1,
            "eventType":"web_execution_completed",
            "timestampMs":2,
            "data":{
              "status":"approval_required",
              "approved":false,
              "degraded":true,
              "query":"search the web for release status"
            }
          },{
            "sequence":2,
            "eventType":"stop_condition_reached",
            "timestampMs":2,
            "data":{"stopReason":"completed"}
          }]
        }"#,
    )
    .expect("trace should write");

    let trace_command = format!(
        "/trace summary {}",
        trace_path.to_str().expect("utf8 trace path")
    );
    let output = run_claw(
        &temp_dir,
        &[
            "--resume",
            session_path.to_str().expect("utf8 path"),
            &trace_command,
        ],
    );

    assert!(
        output.status.success(),
        "stdout:\n{}\n\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Trace"));
    assert!(stdout.contains("Operator state   awaiting approval"));
    assert!(stdout.contains("Pending queries  search the web for release status"));
    assert!(
        stdout.contains("Next step        approve web queries: search the web for release status")
    );
}

#[test]
fn resumed_trace_approve_writes_operator_packet_and_rerun_guidance() {
    let temp_dir = unique_temp_dir("resume-trace-approve");
    let project_dir = temp_dir.join("project");
    fs::create_dir_all(project_dir.join(".claw").join("trace")).expect("trace dir should exist");
    fs::create_dir_all(project_dir.join(".claw").join("corpora")).expect("corpus dir should exist");

    let sessions_dir = project_dir.join(".claw").join("sessions");
    fs::create_dir_all(&sessions_dir).expect("sessions dir should exist");
    let session_path = sessions_dir.join("session.jsonl");
    Session::new()
        .with_persistence_path(&session_path)
        .save_to_path(&session_path)
        .expect("session should persist");

    let corpus_manifest_path = project_dir
        .join(".claw")
        .join("corpora")
        .join("demo-corpus.json");
    fs::write(
        &corpus_manifest_path,
        r#"{
          "artifactKind":"claw.corpus.manifest",
          "schemaVersion":1,
          "compatVersion":"1",
          "corpusId":"demo-corpus",
          "roots":[],
          "kind":"docs",
          "backend":"lexical",
          "documentCount":0,
          "chunkCount":0,
          "estimatedBytes":0,
          "rootSummaries":[],
          "skipSummary":{"skippedRoots":0,"skippedFiles":0,"reasons":[]},
          "documents":[]
        }"#,
    )
    .expect("manifest should write");

    let trace_path = project_dir.join(".claw").join("trace").join("trace.json");
    fs::write(
        &trace_path,
        r#"{
          "traceId":"trace-approval",
          "sessionId":"session-1",
          "rootTaskId":"task-1",
          "startedAtMs":1,
          "finishedAtMs":2,
          "finalStatus":"succeeded",
          "events":[{
            "sequence":1,
            "eventType":"task_started",
            "timestampMs":1,
            "data":{"task":"search the web for release status"}
          },{
            "sequence":2,
            "eventType":"corpus_peeked",
            "timestampMs":1,
            "data":{"corpusId":"demo-corpus"}
          },{
            "sequence":3,
            "eventType":"web_execution_completed",
            "timestampMs":2,
            "data":{
              "status":"approval_required",
              "approved":false,
              "degraded":true,
              "query":"search the web for release status"
            }
          }]
        }"#,
    )
    .expect("trace should write");

    let trace_command = format!(
        "/trace approve {}",
        trace_path.to_str().expect("utf8 trace path")
    );
    let output = run_claw(
        &project_dir,
        &[
            "--resume",
            session_path.to_str().expect("utf8 path"),
            &trace_command,
        ],
    );

    assert!(
        output.status.success(),
        "stdout:\n{}\n\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Trace approval"));
    assert!(stdout.contains("Review JSON      "));
    assert!(stdout.contains("Review Markdown  "));
    assert!(stdout.contains("Review HTML      "));
    assert!(stdout.contains("Review Status    "));
    assert!(stdout.contains("Review Log       "));
    assert!(stdout.contains("Pending queries  search the web for release status"));
    assert!(stdout.contains("Replay command   claw --resume"));
    assert!(stdout.contains("Replay trace     /trace replay"));
    assert!(stdout.contains("/corpus answer demo-corpus :: search the web for release status"));
    assert!(stdout.contains("browser automation is still not available"));

    let packet_path = stdout
        .lines()
        .find_map(|line| line.strip_prefix("  Packet           "))
        .map(PathBuf::from)
        .expect("packet path should be printed");
    let packet = fs::read_to_string(&packet_path).expect("packet should exist");
    let packet_json: JsonValue = serde_json::from_str(&packet).expect("packet json should parse");
    assert_eq!(packet_json["traceId"], "trace-approval");
    assert_eq!(packet_json["corpusId"], "demo-corpus");
    assert_eq!(packet_json["task"], "search the web for release status");
    assert_eq!(
        packet_json["pendingQueries"][0],
        "search the web for release status"
    );
    assert!(packet_json["replayCommand"]
        .as_str()
        .expect("replay command should exist")
        .contains("/corpus answer demo-corpus :: search the web for release status"));

    let review_json_path = stdout
        .lines()
        .find_map(|line| line.strip_prefix("  Review JSON      "))
        .map(PathBuf::from)
        .expect("review json path should be printed");
    let review_json: JsonValue = serde_json::from_str(
        &fs::read_to_string(&review_json_path).expect("review json should exist"),
    )
    .expect("review json should parse");
    assert_eq!(review_json["operatorState"], "approved for explicit rerun");
    assert_eq!(review_json["replayTrace"], JsonValue::Null);
    assert!(review_json["reviewCommand"]
        .as_str()
        .unwrap()
        .contains("/trace review"));
    assert!(review_json["replayTraceCommand"]
        .as_str()
        .unwrap()
        .contains("/trace replay"));
    assert!(review_json["resumeTraceCommand"]
        .as_str()
        .unwrap()
        .contains("/trace resume"));

    let review_markdown_path = stdout
        .lines()
        .find_map(|line| line.strip_prefix("  Review Markdown  "))
        .map(PathBuf::from)
        .expect("review markdown path should be printed");
    let review_markdown =
        fs::read_to_string(&review_markdown_path).expect("review markdown should exist");
    assert!(review_markdown.contains("# Web approval review"));
    assert!(review_markdown.contains("Replay trace: `not yet rerun`"));
    assert!(review_markdown.contains("## Operator commands"));
    assert!(review_markdown.contains("/trace replay "));
    assert!(review_markdown.contains("/trace resume "));

    let review_html_path = stdout
        .lines()
        .find_map(|line| line.strip_prefix("  Review HTML      "))
        .map(PathBuf::from)
        .expect("review html path should be printed");
    let review_html = fs::read_to_string(&review_html_path).expect("review html should exist");
    assert!(review_html.contains("<h1>Web approval review</h1>"));
    assert!(review_html.contains("Static operator web surface only"));
    assert!(review_html.contains("<h2>Operator commands</h2>"));
    assert!(review_html.contains("/trace review "));

    let review_status_path = stdout
        .lines()
        .find_map(|line| line.strip_prefix("  Review Status    "))
        .map(PathBuf::from)
        .expect("review status path should be printed");
    let review_status: JsonValue = serde_json::from_str(
        &fs::read_to_string(&review_status_path).expect("review status should exist"),
    )
    .expect("review status should parse");
    assert_eq!(
        review_status["latestOperatorState"],
        "approved for explicit rerun"
    );
    assert_eq!(review_status["replayCount"], 0);
    assert_eq!(review_status["history"].as_array().unwrap().len(), 1);

    let review_log_path = stdout
        .lines()
        .find_map(|line| line.strip_prefix("  Review Log       "))
        .map(PathBuf::from)
        .expect("review log path should be printed");
    let review_log = fs::read_to_string(&review_log_path).expect("review log should exist");
    assert!(review_log.contains("# Web approval lifecycle log"));
    assert!(review_log.contains("Latest operator state: approved for explicit rerun"));
}

#[test]
fn resumed_trace_replay_updates_review_artifacts_with_rerun_trace() {
    let temp_dir = unique_temp_dir("resume-trace-replay-review");
    let project_dir = temp_dir.join("project");
    fs::create_dir_all(project_dir.join(".claw").join("trace")).expect("trace dir should exist");
    fs::create_dir_all(project_dir.join(".claw").join("corpora")).expect("corpus dir should exist");
    fs::create_dir_all(project_dir.join(".claw").join("sessions"))
        .expect("sessions dir should exist");
    fs::create_dir_all(project_dir.join(".claw").join("web-approvals"))
        .expect("approvals dir should exist");

    let session_path = project_dir
        .join(".claw")
        .join("sessions")
        .join("session.jsonl");
    Session::new()
        .with_persistence_path(&session_path)
        .save_to_path(&session_path)
        .expect("session should persist");

    fs::write(
        project_dir
            .join(".claw")
            .join("corpora")
            .join("demo-corpus.json"),
        r#"{
          "artifactKind":"claw.corpus.manifest",
          "schemaVersion":1,
          "compatVersion":"1",
          "corpusId":"demo-corpus",
          "roots":[],
          "kind":"docs",
          "backend":"lexical",
          "documentCount":0,
          "chunkCount":0,
          "estimatedBytes":0,
          "rootSummaries":[],
          "skipSummary":{"skippedRoots":0,"skippedFiles":0,"reasons":[]},
          "documents":[]
        }"#,
    )
    .expect("manifest should write");

    let trace_path = project_dir.join(".claw").join("trace").join("trace.json");
    fs::write(
        &trace_path,
        r#"{
          "traceId":"trace-approval",
          "sessionId":"session-1",
          "rootTaskId":"task-1",
          "startedAtMs":1,
          "finishedAtMs":2,
          "finalStatus":"succeeded",
          "events":[{
            "sequence":1,
            "eventType":"task_started",
            "timestampMs":1,
            "data":{"task":"search the web for release status"}
          },{
            "sequence":2,
            "eventType":"corpus_peeked",
            "timestampMs":1,
            "data":{"corpusId":"demo-corpus"}
          },{
            "sequence":3,
            "eventType":"web_execution_completed",
            "timestampMs":2,
            "data":{
              "status":"approval_required",
              "approved":false,
              "degraded":true,
              "query":"search the web for release status"
            }
          }]
        }"#,
    )
    .expect("trace should write");

    let packet_path = project_dir
        .join(".claw")
        .join("web-approvals")
        .join("trace-approval.json");
    fs::write(
        &packet_path,
        r#"{
          "schemaVersion":1,
          "traceId":"trace-approval",
          "sessionId":"session-1",
          "task":"search the web for release status",
          "corpusId":"demo-corpus",
          "pendingQueries":["search the web for release status"],
          "approvedAtMs":123,
          "replayCommand":"claw --resume latest \"/corpus answer demo-corpus :: search the web for release status\"",
          "operatorNote":"bounded rerun only"
        }"#,
    )
    .expect("packet should write");

    let trace_command = format!(
        "/trace replay {}",
        trace_path.to_str().expect("utf8 trace path")
    );
    let output = run_claw(
        &project_dir,
        &[
            "--resume",
            session_path.to_str().expect("utf8 path"),
            &trace_command,
        ],
    );
    assert!(
        output.status.success(),
        "stdout:\n{}\n\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Trace replay"));
    assert!(stdout.contains("Review JSON      "));
    assert!(stdout.contains("Review Markdown  "));
    assert!(stdout.contains("Review HTML      "));
    assert!(stdout.contains("Review Status    "));
    assert!(stdout.contains("Review Log       "));
    assert!(stdout.contains("Replay trace     "));

    let review_json_path = stdout
        .lines()
        .find_map(|line| line.strip_prefix("  Review JSON      "))
        .map(PathBuf::from)
        .expect("review json path should be printed");
    let review_json: JsonValue = serde_json::from_str(
        &fs::read_to_string(&review_json_path).expect("review json should exist"),
    )
    .expect("review json should parse");
    assert_eq!(review_json["operatorState"], "rerun captured for review");
    assert!(review_json["replayTrace"].as_str().is_some());
    assert!(review_json["reviewCommand"]
        .as_str()
        .unwrap()
        .contains("/trace review"));

    let review_markdown_path = stdout
        .lines()
        .find_map(|line| line.strip_prefix("  Review Markdown  "))
        .map(PathBuf::from)
        .expect("review markdown path should be printed");
    let review_markdown =
        fs::read_to_string(&review_markdown_path).expect("review markdown should exist");
    assert!(review_markdown.contains("# Web approval review"));
    assert!(!review_markdown.contains("Replay trace: `not yet rerun`"));
    assert!(review_markdown.contains("## Operator commands"));

    let review_html_path = stdout
        .lines()
        .find_map(|line| line.strip_prefix("  Review HTML      "))
        .map(PathBuf::from)
        .expect("review html path should be printed");
    let review_html = fs::read_to_string(&review_html_path).expect("review html should exist");
    assert!(review_html.contains("Web approval review"));
    assert!(review_html.contains("Replay trace"));
    assert!(review_html.contains("Operator commands"));

    let review_status_path = stdout
        .lines()
        .find_map(|line| line.strip_prefix("  Review Status    "))
        .map(PathBuf::from)
        .expect("review status path should be printed");
    let review_status: JsonValue = serde_json::from_str(
        &fs::read_to_string(&review_status_path).expect("review status should exist"),
    )
    .expect("review status should parse");
    assert_eq!(
        review_status["latestOperatorState"],
        "rerun captured for review"
    );
    assert_eq!(review_status["replayCount"], 1);
    assert_eq!(review_status["history"].as_array().unwrap().len(), 1);
}

#[test]
fn resumed_trace_resume_approves_reruns_and_refreshes_review_index() {
    let temp_dir = unique_temp_dir("resume-trace-approve-and-rerun");
    let project_dir = temp_dir.join("project");
    fs::create_dir_all(project_dir.join(".claw").join("trace")).expect("trace dir should exist");
    fs::create_dir_all(project_dir.join(".claw").join("corpora")).expect("corpus dir should exist");
    fs::create_dir_all(project_dir.join(".claw").join("sessions"))
        .expect("sessions dir should exist");

    let session_path = project_dir
        .join(".claw")
        .join("sessions")
        .join("session.jsonl");
    Session::new()
        .with_persistence_path(&session_path)
        .save_to_path(&session_path)
        .expect("session should persist");

    fs::write(
        project_dir
            .join(".claw")
            .join("corpora")
            .join("demo-corpus.json"),
        r#"{
          "artifactKind":"claw.corpus.manifest",
          "schemaVersion":1,
          "compatVersion":"1",
          "corpusId":"demo-corpus",
          "roots":[],
          "kind":"docs",
          "backend":"lexical",
          "documentCount":0,
          "chunkCount":0,
          "estimatedBytes":0,
          "rootSummaries":[],
          "skipSummary":{"skippedRoots":0,"skippedFiles":0,"reasons":[]},
          "documents":[]
        }"#,
    )
    .expect("manifest should write");

    let trace_path = project_dir.join(".claw").join("trace").join("trace.json");
    fs::write(
        &trace_path,
        r#"{
          "traceId":"trace-approval",
          "sessionId":"session-1",
          "rootTaskId":"task-1",
          "startedAtMs":1,
          "finishedAtMs":2,
          "finalStatus":"succeeded",
          "events":[{
            "sequence":1,
            "eventType":"task_started",
            "timestampMs":1,
            "data":{"task":"search the web for release status"}
          },{
            "sequence":2,
            "eventType":"corpus_peeked",
            "timestampMs":1,
            "data":{"corpusId":"demo-corpus"}
          },{
            "sequence":3,
            "eventType":"web_execution_completed",
            "timestampMs":2,
            "data":{
              "status":"approval_required",
              "approved":false,
              "degraded":true,
              "query":"search the web for release status"
            }
          }]
        }"#,
    )
    .expect("trace should write");

    let trace_command = format!(
        "/trace resume {}",
        trace_path.to_str().expect("utf8 trace path")
    );
    let output = run_claw(
        &project_dir,
        &[
            "--resume",
            session_path.to_str().expect("utf8 path"),
            &trace_command,
        ],
    );

    assert!(
        output.status.success(),
        "stdout:\n{}\n\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Trace resume"));
    assert!(stdout.contains("approval recorded and rerun executed"));
    assert!(stdout.contains("Review Index JSON"));
    assert!(stdout.contains("Review Index MD"));
    assert!(stdout.contains("Review Index HTML"));
    assert!(stdout.contains("Review Status    "));
    assert!(stdout.contains("Review Log       "));

    let review_json_path = stdout
        .lines()
        .find_map(|line| line.strip_prefix("  Review JSON      "))
        .map(PathBuf::from)
        .expect("review json path should be printed");
    let review_json: JsonValue = serde_json::from_str(
        &fs::read_to_string(&review_json_path).expect("review json should exist"),
    )
    .expect("review json should parse");
    assert_eq!(review_json["operatorState"], "rerun captured for review");
    assert!(review_json["replayTrace"].as_str().is_some());

    let index_json_path = stdout
        .lines()
        .find_map(|line| line.strip_prefix("  Review Index JSON "))
        .map(PathBuf::from)
        .expect("index json path should be printed");
    let index_json: JsonValue = serde_json::from_str(
        &fs::read_to_string(&index_json_path).expect("index json should exist"),
    )
    .expect("index json should parse");
    let entries = index_json["entries"]
        .as_array()
        .expect("entries should be an array");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["traceId"], "trace-approval");
    assert_eq!(entries[0]["operatorState"], "rerun captured for review");
    assert_eq!(entries[0]["replayCount"], 1);
    assert_eq!(index_json["summary"]["replayCount"], 1);

    let index_markdown_path = stdout
        .lines()
        .find_map(|line| line.strip_prefix("  Review Index MD  "))
        .map(PathBuf::from)
        .expect("index markdown path should be printed");
    let index_markdown =
        fs::read_to_string(&index_markdown_path).expect("index markdown should exist");
    assert!(index_markdown.contains("# Web approval review index"));
    assert!(index_markdown.contains("trace `trace-approval` — rerun captured for review"));
    assert!(index_markdown.contains("review command: /trace review"));
    assert!(index_markdown.contains("resume command: /trace resume"));
    assert!(index_markdown.contains("review status:"));
    assert!(index_markdown.contains("review log:"));
    assert!(index_markdown.contains("replay count: 1"));
    assert!(index_markdown.contains("not a browser UI or automation surface"));

    let index_html_path = stdout
        .lines()
        .find_map(|line| line.strip_prefix("  Review Index HTML "))
        .map(PathBuf::from)
        .expect("index html path should be printed");
    let index_html = fs::read_to_string(&index_html_path).expect("index html should exist");
    assert!(index_html.contains("<h1>Web approval dashboard</h1>"));
    assert!(index_html.contains("Static review surface generated on disk"));
    assert!(index_html.contains("Operator commands"));
    assert!(index_html.contains("Replay count"));
}

#[test]
fn resumed_trace_review_accepts_bare_trace_id_and_reports_lifecycle_paths() {
    let temp_dir = unique_temp_dir("resume-trace-review-by-id");
    let project_dir = temp_dir.join("project");
    fs::create_dir_all(project_dir.join(".claw").join("sessions"))
        .expect("sessions dir should exist");
    fs::create_dir_all(project_dir.join(".claw").join("web-approvals"))
        .expect("approvals dir should exist");

    let session_path = project_dir
        .join(".claw")
        .join("sessions")
        .join("session.jsonl");
    Session::new()
        .with_persistence_path(&session_path)
        .save_to_path(&session_path)
        .expect("session should persist");

    let packet_path = project_dir
        .join(".claw")
        .join("web-approvals")
        .join("trace-approval.json");
    fs::write(
        &packet_path,
        r#"{
          "schemaVersion":1,
          "traceId":"trace-approval",
          "sessionId":"session-1",
          "task":"search the web for release status",
          "corpusId":"demo-corpus",
          "pendingQueries":["search the web for release status"],
          "approvedAtMs":123,
          "replayCommand":"claw --resume latest \"/corpus answer demo-corpus :: search the web for release status\"",
          "operatorNote":"bounded rerun only"
        }"#,
    )
    .expect("packet should write");
    fs::write(
        packet_path.with_extension("review.json"),
        r#"{
          "schemaVersion":1,
          "traceId":"trace-approval",
          "operatorState":"approved for explicit rerun",
          "nextStep":"run /trace replay <trace-file>",
          "replayTrace":null
        }"#,
    )
    .expect("review json should write");
    fs::write(
        packet_path.with_extension("review-status.json"),
        r#"{
          "schemaVersion":1,
          "traceId":"trace-approval",
          "latestOperatorState":"approved for explicit rerun",
          "latestNextStep":"run /trace replay <trace-file>",
          "replayCount":0,
          "history":[{"recordedAtMs":123,"operatorState":"approved for explicit rerun"}]
        }"#,
    )
    .expect("review status should write");
    fs::write(
        packet_path.with_extension("review-log.md"),
        "# Web approval lifecycle log\n",
    )
    .expect("review log should write");

    let output = run_claw(
        &project_dir,
        &[
            "--resume",
            session_path.to_str().expect("utf8 path"),
            "/trace review trace-approval",
        ],
    );
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Trace review"));
    assert!(stdout.contains("Review Status    "));
    assert!(stdout.contains("Review Log       "));
    assert!(stdout.contains("Operator state   approved for explicit rerun"));
    assert!(stdout.contains("Lifecycle entries 1"));
    assert!(stdout.contains("Replay count     0"));
}

#[test]
fn resumed_trace_review_reports_pending_or_rerun_state() {
    let temp_dir = unique_temp_dir("resume-trace-review");
    let project_dir = temp_dir.join("project");
    fs::create_dir_all(project_dir.join(".claw").join("trace")).expect("trace dir should exist");
    fs::create_dir_all(project_dir.join(".claw").join("sessions"))
        .expect("sessions dir should exist");
    fs::create_dir_all(project_dir.join(".claw").join("web-approvals"))
        .expect("approvals dir should exist");

    let session_path = project_dir
        .join(".claw")
        .join("sessions")
        .join("session.jsonl");
    Session::new()
        .with_persistence_path(&session_path)
        .save_to_path(&session_path)
        .expect("session should persist");

    let trace_path = project_dir.join(".claw").join("trace").join("trace.json");
    fs::write(&trace_path, "{}").expect("placeholder trace should write");

    let packet_path = project_dir
        .join(".claw")
        .join("web-approvals")
        .join("trace-approval.json");
    fs::write(
        &packet_path,
        r#"{
          "schemaVersion":1,
          "traceId":"trace-approval",
          "sessionId":"session-1",
          "task":"search the web for release status",
          "corpusId":"demo-corpus",
          "pendingQueries":["search the web for release status"],
          "approvedAtMs":123,
          "replayCommand":"claw --resume latest \"/corpus answer demo-corpus :: search the web for release status\"",
          "operatorNote":"bounded rerun only"
        }"#,
    )
    .expect("packet should write");
    fs::write(
        packet_path.with_extension("review.json"),
        r#"{
          "schemaVersion":1,
          "traceId":"trace-approval",
          "operatorState":"approved for explicit rerun",
          "nextStep":"run /trace replay <trace-file>",
          "replayTrace":null
        }"#,
    )
    .expect("review json should write");

    let trace_command = format!("/trace review {}", packet_path.to_str().expect("utf8 path"));
    let output = run_claw(
        &project_dir,
        &[
            "--resume",
            session_path.to_str().expect("utf8 path"),
            &trace_command,
        ],
    );
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Trace review"));
    assert!(stdout.contains("Review Markdown  "));
    assert!(stdout.contains("Review HTML      "));
    assert!(stdout.contains("Operator state   approved for explicit rerun"));
    assert!(stdout.contains("Lifecycle entries 0"));
    assert!(stdout.contains("Replay count     0"));
    assert!(stdout.contains("Replay trace     <not yet rerun>"));
}

#[test]
fn resumed_trace_approvals_dashboard_lists_review_entries() {
    let temp_dir = unique_temp_dir("resume-trace-approvals-dashboard");
    let project_dir = temp_dir.join("project");
    fs::create_dir_all(project_dir.join(".claw").join("sessions"))
        .expect("sessions dir should exist");
    fs::create_dir_all(project_dir.join(".claw").join("web-approvals"))
        .expect("approvals dir should exist");

    let session_path = project_dir
        .join(".claw")
        .join("sessions")
        .join("session.jsonl");
    Session::new()
        .with_persistence_path(&session_path)
        .save_to_path(&session_path)
        .expect("session should persist");

    fs::write(
        project_dir
            .join(".claw")
            .join("web-approvals")
            .join("trace-approval.review.json"),
        r#"{
          "schemaVersion":1,
          "traceId":"trace-approval",
          "corpusId":"demo-corpus",
          "task":"search the web for release status",
          "approvalPacket":"/tmp/trace-approval.json",
          "operatorState":"rerun captured for review",
          "nextStep":"inspect replay trace",
          "replayTrace":"/tmp/replay-trace.json",
          "pendingQueries":["search the web for release status"]
        }"#,
    )
    .expect("review json should write");

    let output = run_claw(
        &project_dir,
        &[
            "--resume",
            session_path.to_str().expect("utf8 path"),
            "/trace approvals",
        ],
    );
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Trace approvals"));
    assert!(stdout.contains("Review Index HTML  "));
    assert!(stdout.contains("Entries            1"));
    assert!(stdout.contains("Rerun captured     1"));
    assert!(stdout.contains("Pending queries    1"));
    assert!(stdout.contains("Recorded replays   0"));
    assert!(stdout.contains("trace-approval :: rerun captured for review"));
    assert!(stdout.contains("task: search the web for release status"));
    assert!(stdout.contains("corpus: demo-corpus"));
    assert!(stdout.contains("packet: /tmp/trace-approval.json"));
    assert!(stdout.contains("trace-approval :: rerun captured for review"));

    let index_markdown = fs::read_to_string(
        project_dir
            .join(".claw")
            .join("web-approvals")
            .join("index.md"),
    )
    .expect("index markdown should exist");
    assert!(index_markdown.contains("## Summary"));
    assert!(index_markdown.contains("- Pending approved queries: 1"));
    assert!(index_markdown.contains("- task: search the web for release status"));
    assert!(index_markdown.contains("- corpus: demo-corpus"));
    assert!(index_markdown.contains("trace-approval"));

    let index_html = fs::read_to_string(
        project_dir
            .join(".claw")
            .join("web-approvals")
            .join("index.html"),
    )
    .expect("index html should exist");
    assert!(index_html.contains("Web approval dashboard"));
    assert!(index_html.contains("trace-approval"));
    assert!(index_html.contains("Operator commands"));
}

#[test]
fn resume_latest_restores_the_most_recent_managed_session() {
    // given
    let temp_dir = unique_temp_dir("resume-latest");
    let project_dir = temp_dir.join("project");
    let sessions_dir = project_dir.join(".claw").join("sessions");
    fs::create_dir_all(&sessions_dir).expect("sessions dir should exist");

    let older_path = sessions_dir.join("session-older.jsonl");
    let newer_path = sessions_dir.join("session-newer.jsonl");

    let mut older = Session::new().with_persistence_path(&older_path);
    older
        .push_user_text("older session")
        .expect("older session write should succeed");
    older
        .save_to_path(&older_path)
        .expect("older session should persist");

    let mut newer = Session::new().with_persistence_path(&newer_path);
    newer
        .push_user_text("newer session")
        .expect("newer session write should succeed");
    newer
        .push_user_text("resume me")
        .expect("newer session write should succeed");
    newer
        .save_to_path(&newer_path)
        .expect("newer session should persist");

    // when
    let output = run_claw(&project_dir, &["--resume", "latest", "/status"]);

    // then
    assert!(
        output.status.success(),
        "stdout:\n{}\n\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Status"));
    assert!(stdout.contains("Messages         2"));
    assert!(stdout.contains(newer_path.to_str().expect("utf8 path")));
}

fn run_claw(current_dir: &Path, args: &[&str]) -> Output {
    run_claw_with_env(current_dir, args, &[])
}

fn run_claw_with_env(current_dir: &Path, args: &[&str], envs: &[(&str, &str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_claw"));
    command.current_dir(current_dir).args(args);
    for (key, value) in envs {
        command.env(key, value);
    }
    command.output().expect("claw should launch")
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after epoch")
        .as_millis();
    let counter = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "claw-{label}-{}-{millis}-{counter}",
        std::process::id()
    ))
}

#[test]
fn resumed_trace_inbox_prioritizes_next_actionable_entry() {
    let temp_dir = unique_temp_dir("resume-trace-inbox");
    let project_dir = temp_dir.join("project");
    fs::create_dir_all(project_dir.join(".claw").join("sessions"))
        .expect("sessions dir should exist");
    fs::create_dir_all(project_dir.join(".claw").join("web-approvals"))
        .expect("approvals dir should exist");

    let session_path = project_dir
        .join(".claw")
        .join("sessions")
        .join("session.jsonl");
    Session::new()
        .with_persistence_path(&session_path)
        .save_to_path(&session_path)
        .expect("session should persist");

    fs::write(
        project_dir
            .join(".claw")
            .join("web-approvals")
            .join("trace-approved.review.json"),
        r#"{
          "schemaVersion":1,
          "traceId":"trace-approved",
          "corpusId":"demo-corpus",
          "task":"rerun the approved release-status search",
          "approvalPacket":"/tmp/trace-approved.json",
          "operatorState":"approved for explicit rerun",
          "nextStep":"run /trace replay <trace-file>",
          "replayTrace":null,
          "reviewCommand":"/trace review /tmp/trace-approved.json",
          "replayTraceCommand":"/trace replay /tmp/trace-approved.json",
          "resumeTraceCommand":"/trace resume /tmp/trace-approved.json",
          "pendingQueries":["search the web for release status"]
        }"#,
    )
    .expect("approved review json should write");
    fs::write(
        project_dir
            .join(".claw")
            .join("web-approvals")
            .join("trace-captured.review.json"),
        r#"{
          "schemaVersion":1,
          "traceId":"trace-captured",
          "corpusId":"demo-corpus",
          "task":"inspect the replay trace",
          "approvalPacket":"/tmp/trace-captured.json",
          "operatorState":"rerun captured for review",
          "nextStep":"inspect replay trace",
          "replayTrace":"/tmp/replay-trace.json",
          "reviewCommand":"/trace review /tmp/trace-captured.json",
          "replayTraceCommand":"/trace replay /tmp/trace-captured.json",
          "resumeTraceCommand":"/trace resume /tmp/trace-captured.json",
          "pendingQueries":["search the web for release status"]
        }"#,
    )
    .expect("captured review json should write");

    let output = run_claw(
        &project_dir,
        &[
            "--resume",
            session_path.to_str().expect("utf8 path"),
            "/trace inbox",
        ],
    );
    assert!(
        output.status.success(),
        "stdout:\n{}\n\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Trace inbox"), "stdout:\n{}", stdout);
    assert!(stdout.contains("Ready to rerun     1"));
    assert!(stdout.contains("Ready to review    1"));
    assert!(stdout.contains("trace-approved :: Ready to rerun"));
    assert!(stdout.contains("replay: /trace replay /tmp/trace-approved.json"));
    assert!(stdout.contains("resume: /trace resume /tmp/trace-approved.json"));

    let index_json: JsonValue = serde_json::from_str(
        &fs::read_to_string(
            project_dir
                .join(".claw")
                .join("web-approvals")
                .join("index.json"),
        )
        .expect("index json should exist"),
    )
    .expect("index json should parse");
    assert_eq!(index_json["summary"]["readyToRerun"], 1);
    assert_eq!(index_json["summary"]["readyToReview"], 1);
    let entries = index_json["entries"]
        .as_array()
        .expect("entries should exist");
    assert_eq!(entries[0]["traceId"], "trace-approved");
    assert_eq!(entries[0]["queueBucket"], "ready-to-rerun");
    assert_eq!(entries[1]["queueBucket"], "ready-to-review");
}
