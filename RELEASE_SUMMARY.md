# Release Summary

## Current State
The repository is currently best described as a **strong alpha / early pre-production candidate**.

It is no longer just a parity snapshot. The fork now includes substantial additional work around:
- local corpus RAG
- recursive runtime behavior
- provider-backed child subqueries
- trace export and telemetry
- execution profiles
- web-aware policy/provenance handling
- workflow recipes and demo kits
- release/trust/operator docs

## What improved in this batch
### Core runtime
- stronger recursive stop semantics (`NoNewContext`, `Converged`)
- better child failure handling
- partial modularization of the recursive runtime under `rust/crates/runtime/src/rlm/`
- stronger stop-event metadata and trace consistency

### Child/provider execution
- cleaner shared provider child backend/factory direction
- less CLI-owned duplication
- shared provider extractive child executor builder moved into the `api` crate
- model resolution and bounded minimal web-evidence shaping were pushed further into shared `api` helpers
- auth resolution and default bounded web adapter paths were pushed further into shared `api` helpers
- a shared provider recursive runtime builder now removes one more orchestration seam from the CLI layer
- provider-backed recursive query orchestration now owns more budget/policy/telemetry/trace setup in shared `api` code
- generic provider recursive task orchestration surfaces now exist alongside corpus-answer compatibility wrappers
- better fallback messaging and backend availability reporting

### Local corpus RAG
- richer lexical ranking
- better retrieval explainability
- better slice diversity across documents
- root-aware retrieval provenance
- per-root disambiguation for same-named files
- symbol-aware query heuristics for code-heavy and multi-root retrieval
- bounded semantic and structure-aware query signals for common repo/docs vocabulary
- query-intent routing and document-continuity signals improve file/docs/implementation-style retrieval
- outline-aware and section-aware scoring improve explain/architecture-style retrieval within documents
- skip telemetry / per-root reporting improvements

### Web-aware behavior
- explicit web execution completion events in traces
- richer web counters/telemetry
- child execution path now handles approved web evidence more honestly
- ask-mode now preserves approval-required web query provenance more explicitly
- degraded web collection is surfaced as explicit notes instead of silent failure or overclaiming
- fetch-state-aware provenance distinguishes fetched pages from search-result snippets
- final answers now render a dedicated operator-facing `Web execution` section with per-subquery status/detail summaries
- final-answer web execution summaries are more honest and more detailed
- resumed `/trace` flows now surface pending approval-required web queries and next-step guidance more practically
- provenance handling is stronger

### Operator / product surface
- stronger root and Rust README docs
- bootstrap / quickstart guidance
- release/trust docs (`CHANGELOG`, `RELEASE`, `ARTIFACTS`, `PRIVACY`, `RELEASE_CANDIDATE`)
- production-readiness planning docs, checklist, scorecard, and implementation plan
- stronger RC-readiness validation in tests and release verification flow
- machine-readable release artifact manifests with local hashing/verification for key release surfaces
- domain workflow docs, prompt templates, demo kits, lightweight readiness validators, a runnable repo-analysis showcase helper, honest Blender/Unreal prep helpers, and a stronger Unreal operator handoff runbook

### Showcase workflows
- Blender scene cleanup demo kit
- Unreal runtime telemetry demo kit
- Repo analysis demo kit

## Honest limitations
This repository is **not yet fully production-ready**.

Main reasons:
- web execution is more honest and better traced, but still not a fully mature end-to-end runtime path
- child execution is cleaner and more centralized, but still not perfectly owned by shared runtime/provider abstractions
- retrieval is stronger and more symbol-aware, but still remains heuristic/lexical-first overall
- recursive planning remains alpha, even though it is now much stronger than the baseline
- workflow validation for Blender/Unreal remains operator-driven rather than fully automated, even though repo analysis is now a better runnable showcase path

## Recommended positioning
Use language like:

> Rust-first agent harness with real local corpus RAG, improving recursive runtime behavior, stronger traceability, and early hybrid local+web support. Suitable for serious local technical experimentation and grounded custom-task workflows, but not yet a fully production-ready universal agent platform.

## Verified state
Using the pinned Rust toolchain from the real workspace root at `rust/`, the current repository state verifies with:
- `cargo build --workspace --locked` ✅
- `cargo test --workspace --locked` ✅

## Suggested next focus
1. finalize release-candidate cleanliness and locked verification flow
2. continue moving child execution fully into shared runtime/provider abstractions
3. strengthen the minimal web executor into a more trustworthy path
4. improve retrieval quality beyond lexical-only limits
5. keep one showcase workflow (Blender) polished as the main proof path
