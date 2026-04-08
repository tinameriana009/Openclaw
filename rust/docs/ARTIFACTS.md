# Runtime artifact contract notes

This document explains the on-disk artifacts the Rust harness writes under `.claw/` today, what operators can safely rely on, and where the compatibility boundaries still are.

## Scope

Current operator-relevant artifact locations:

- `.claw/trace/` — recursive trace ledgers
- `.claw/telemetry/recursive-runtime.jsonl` — recursive runtime telemetry stream
- `.claw/sessions/` — saved local sessions
- `.claw/corpora/` — persisted corpus manifests
- `.claw/release-artifacts/release-manifest.json` — machine-readable release/build artifact manifest for the current verified workspace

These files are useful for inspection, debugging, evaluation, and incident review.

## Contract level

The current workspace version is `0.1.0`.

That means:

- file formats are real and intentionally documented
- operators can inspect them directly today
- automation should still treat them as **pre-1.0 contracts**
- minor field additions are plausible without a dedicated migration layer yet

Practical rule:

- **Human inspection and internal tooling:** reasonable today
- **Strict third-party parsers:** pin to a tagged version / commit and validate defensively

## Artifact envelope metadata

Trace ledgers and corpus manifests now carry a lightweight envelope:

- `artifactKind` — identifies the artifact family (`claw.trace-ledger` or `claw.corpus-manifest`)
- `schemaVersion` — artifact-specific schema version
- `compatVersion` — coarse compatibility line for operator tooling

Current values in `0.1.0`:

- traces: `artifactKind=claw.trace-ledger`, `schemaVersion=1`, `compatVersion=0.1`
- corpora: `artifactKind=claw.corpus-manifest`, `schemaVersion=1`, `compatVersion=0.1`

Reader behavior today:

- current runtime writes the envelope fields for new trace/corpus artifacts
- current runtime still reads older unversioned trace/corpus JSON defensively
- missing envelope fields should be interpreted as legacy local artifacts, not as proof of corruption

## Trace ledgers

Trace ledgers are saved as JSON objects under `.claw/trace/`.

From `runtime/src/trace.rs`, the top-level ledger currently contains:

- `artifactKind`
- `schemaVersion`
- `compatVersion`
- `traceId`
- `sessionId`
- `rootTaskId`
- `startedAtMs`
- `finishedAtMs`
- `finalStatus`
- `events`

Current `finalStatus` values:

- `running`
- `succeeded`
- `failed`
- `cancelled`
- `budget_exceeded`

Current event types include:

- `task_started`
- `retrieval_requested`
- `retrieval_completed`
- `corpus_peeked`
- `corpus_sliced`
- `subquery_started`
- `subquery_completed`
- `web_escalation_started`
- `web_evidence_added`
- `aggregation_completed`
- `stop_condition_reached`
- `task_failed`

### Important trust note

A trace ledger is a **structured event log**, not raw hidden chain-of-thought. It is intended to expose execution shape, evidence movement, and stop conditions without promising a verbatim reasoning transcript.

## Corpus manifests

Corpus manifests are saved as JSON objects under `.claw/corpora/`.

Current top-level fields from `runtime/src/corpus.rs`:

- `artifactKind`
- `schemaVersion`
- `compatVersion`
- `corpusId`
- `roots`
- `kind`
- `backend`
- `documentCount`
- `chunkCount`
- `estimatedBytes`
- `rootSummaries`
- `skipSummary`
- `documents`

Document records currently include:

- `documentId`
- `path`
- `mediaType`
- `language`
- `headings`
- `bytes`
- `modifiedAtMs`
- `chunks`

Chunk records currently include:

- `chunkId`
- `documentId`
- `ordinal`
- `startOffset`
- `endOffset`
- `textPreview`
- `metadata`

### Trust note

Corpus manifests intentionally persist metadata plus previews/chunk structure. They should be treated as sensitive if source paths, headings, or previews reveal private repository content.

## Recursive telemetry JSONL

Recursive telemetry is appended to `.claw/telemetry/recursive-runtime.jsonl`.

Operators should treat it as:

- append-oriented event data
- useful for counters, timings, and high-level lifecycle review
- less suitable than saved trace ledgers for long-term compatibility-sensitive automation

JSONL consumers should:

- read line-by-line
- ignore unknown fields
- tolerate future event additions

## Release artifact manifest

`./scripts/generate-release-artifact-manifest.sh` writes `.claw/release-artifacts/release-manifest.json`.

Its current purpose is still bounded but more provenance-aware: after a fresh local build, it records the current workspace version, required toolchain, git commit/branch/dirty state, remote hints, a compact local build environment snapshot, the intended verification command set, and SHA-256/byte metadata for the key operator-facing release artifacts:

- `target/debug/claw`
- `README.md`
- `RELEASE.md`
- `CHANGELOG.md`
- `Cargo.lock`
- `docs/ARTIFACTS.md`
- `docs/PRIVACY.md`
- `docs/RELEASE_CANDIDATE.md`
- `scripts/release-verify.sh`
- `scripts/generate-release-artifact-manifest.sh`

Envelope fields:

- `artifactKind=claw.release-manifest`
- `schemaVersion=2`
- `compatVersion=0.2`

Validation path:

```bash
cd rust
manifest_path=$(./scripts/generate-release-artifact-manifest.sh)
python3 ../tests/validate_release_artifact_manifest.py "$manifest_path"
```

Useful fields beyond the artifact hash list now include:

- `git.statusShort` and `git.remotes` — local posture hints for the exact clone that produced the manifest
- `build.host` — compact OS/toolchain/python context for the machine that built it
- `build.subject` — explicit binding back to the produced `target/debug/claw` binary hash
- `build.materials` — the specific release/trust surfaces this manifest expects operators to preserve together
- `verification.commands` — the locked command set the local build is supposed to pass
- `verification.notes` — explicit reminders that this is still a local/source-build trust aid, not a signed attestation chain

This is **still not** signed provenance and it is **still not** a packaged binary release format. The upgrade here is narrower and more honest: the manifest now carries enough local build/verification context that a maintainer can reconstruct *what was trusted, from which clone, on which host/toolchain, and with which verification routine*. That is better than source-only trust, but still intentionally short of a full artifact attestation system.

## Sessions

Saved sessions under `.claw/sessions/` are the resumable local conversation state.

They are operational artifacts, not polished export contracts. They are useful for:

- resume flows
- local debugging
- incident reconstruction

They should not yet be treated as a durable public interchange format.

## Compatibility guidance

The safest compatibility anchor is still:

1. the git tag or commit
2. the workspace version in `rust/Cargo.toml`
3. the artifact envelope (`artifactKind`, `schemaVersion`, `compatVersion`)
4. defensive parsing that ignores unknown fields

If you are building automation around `.claw/` artifacts, prefer this approach:

- pin to a known release/tag
- validate required keys
- verify `artifactKind` before deeper parsing
- branch on `schemaVersion` if you need strict logic
- use `compatVersion` for coarse operator expectations
- ignore additive keys
- fail clearly on missing required keys or type changes

## Planned hardening still not present

Useful future upgrades that are **not** implemented yet:

- dedicated migration notes for session formats
- stable machine-readable release manifests
- built-in redaction or export-scrubbing helpers
- signed release artifacts or packaged binaries

Until then, use tagged builds and this document together as the practical trust boundary.
