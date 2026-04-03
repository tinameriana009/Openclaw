# Task Tracker — RAG + Recursive Language Model Harness

## Status
Active planning tracker

## Tracker Rules
- `[ ]` not started
- `[-]` in progress
- `[x]` done
- Keep tasks implementation-oriented and testable.
- Prefer small merged slices over giant branches.

## Current Source Docs
- `PRD_RAG_RLM_HARNESS.md`
- `ARCHITECTURE_RAG_RLM.md`

## Milestone Overview
1. Foundation + audit
2. Config + data model scaffolding
3. Corpus indexing + lexical RAG MVP
4. RLM depth-1 controller MVP
5. Hybrid local+web mode
6. UX, traceability, and hardening

---

# M0 — Foundation / Audit

## M0.1 Repo and runtime audit
- [x] Confirm build/test status of current Rust workspace
- [x] Identify exact entrypoints for CLI session startup
- [x] Identify exact insertion points for runtime orchestration wrapper
- [x] Identify exact insertion points for config parsing extensions
- [x] Identify existing telemetry hooks reusable for RLM trace events

### Deliverables
- [x] Build/test audit notes committed
- [x] Concrete module map appended to architecture doc if needed

## M0.2 Execution profiles and defaults
- [ ] Decide default user-facing profiles: `fast`, `balanced`, `deep`, `research`
- [ ] Decide safe default recursion caps
- [ ] Decide safe default web policy
- [ ] Decide default chunk sizing heuristic for v1 lexical backend

### Acceptance
- [ ] Defaults documented in architecture/spec docs

---

# M1 — Config and Core Data Models

## M1.1 Config schema additions
- [x] Add `RuntimeRagConfig` to runtime config model
- [x] Add `RuntimeRlmConfig` to runtime config model
- [x] Add `RuntimeWebResearchConfig` to runtime config model
- [x] Wire them into `RuntimeFeatureConfig`
- [x] Parse typed config from settings JSON
- [x] Add unit tests for config parsing precedence and defaults

### Acceptance
- [x] Config loader accepts `rag`, `rlm`, `webResearch` sections
- [x] Invalid config produces typed parse errors
- [x] Existing config tests still pass

## M1.2 Budget model
- [ ] Add budget structs for depth / iteration / timeout / subcalls / optional cost
- [ ] Add inheritance logic from parent to child budget
- [ ] Add budget exhaustion helpers and stop-reason enum
- [ ] Add unit tests for budget slicing and cap enforcement

### Acceptance
- [ ] Child budget can never exceed parent budget
- [ ] Controller can query exhaustion state deterministically

## M1.3 Trace ledger schema
- [ ] Define trace root record
- [ ] Define event enum / event payload structs
- [ ] Define trace serialization format (JSON recommended)
- [ ] Add helper for emitting high-level summary
- [ ] Add tests for serialization and backward-safe parsing assumptions

### Acceptance
- [ ] A trace file can be written and re-read
- [ ] Event sequence is inspectable without raw transcript dumping

## M1.4 Corpus data model
- [ ] Define corpus / document / chunk structs
- [ ] Define retrieval hit/result structs
- [ ] Define corpus manifest storage layout
- [ ] Add tests for chunk/document id stability

### Acceptance
- [ ] Corpus can represent repo/docs/notes roots in one schema

---

# M2 — Corpus Indexing + Local Lexical RAG MVP

## M2.1 Corpus attachment
- [ ] Implement corpus attachment from local path(s)
- [ ] Detect supported file types for v1
- [ ] Ignore binary/oversized/unsupported files safely
- [ ] Persist corpus metadata under harness data directory

### Acceptance
- [ ] User can attach a local repo/docs directory as a corpus
- [ ] Corpus metadata survives session restart if configured

## M2.2 Chunking pipeline
- [ ] Implement text chunking strategy for code and markdown/plaintext
- [ ] Include metadata: path, language, heading, offsets, chunk ordinal
- [ ] Add chunk preview generation
- [ ] Add tests for chunk boundaries and heading capture

### Acceptance
- [ ] Chunker produces deterministic output for fixture corpora

## M2.3 Lexical retrieval backend
- [ ] Implement simple lexical search over chunks
- [ ] Support path/file-name and content matching
- [ ] Add ranking heuristics for path, heading, hit density
- [ ] Support top-k and optional path filter
- [ ] Add retrieval tests against small corpora

### Acceptance
- [ ] Queries return cited hits with score + preview
- [ ] Retrieval works without embeddings/vector DB

## M2.4 Corpus tools
- [ ] Add `CorpusInspect` tool
- [ ] Add `CorpusSearch` tool
- [ ] Add `CorpusSlice` tool
- [ ] Register tool specs and permission requirements
- [ ] Add tool execution tests

### Acceptance
- [ ] Tools are visible in registry and usable in runtime
- [ ] Tool output is structured and citation-friendly

## M2.5 CLI / slash command surfacing for corpus
- [ ] Add `/corpus` overview command
- [ ] Add `/corpus attach <path>` command
- [ ] Add `/corpus search <query>` command
- [ ] Add any required CLI flags for corpus roots

### Acceptance
- [ ] User can attach and inspect corpus without editing config by hand

---

# M3 — RLM Depth-1 MVP

## M3.1 Root controller scaffold
- [ ] Add `RecursiveConversationRuntime` or equivalent wrapper
- [ ] Add mode selection between direct / RAG / RLM execution
- [ ] Add state struct for recursive execution
- [ ] Add stop-reason enum and controller loop skeleton

### Acceptance
- [ ] Runtime can enter a bounded recursive mode for a task

## M3.2 Context-inspection operations
- [ ] Implement corpus peek operation
- [ ] Implement corpus search operation via retrieval backend
- [ ] Implement slice selection operation
- [ ] Ensure all actions emit trace events

### Acceptance
- [ ] Root loop can inspect corpus without loading all content into root prompt

## M3.3 Child subquery execution
- [ ] Define child subquery input/output schema
- [ ] Implement child call helper using provider API
- [ ] Route narrowed slice content only to child call
- [ ] Add child usage accounting
- [ ] Add unit/integration tests for depth-1 subquery flow

### Acceptance
- [ ] Parent can issue one or more child subqueries and collect structured outputs

## M3.4 Aggregation and stopping
- [ ] Implement parent aggregation over child outputs
- [ ] Implement iteration cap enforcement
- [ ] Implement depth cap enforcement
- [ ] Implement timeout/subcall cap enforcement
- [ ] Surface stop reason in trace and final metadata

### Acceptance
- [ ] Recursive mode terminates cleanly under all configured caps

## M3.5 RLM trace export
- [ ] Write trace file to session artifact path
- [ ] Add `/trace` summary command
- [ ] Add `/trace export [path]` command

### Acceptance
- [ ] User can inspect recursive steps after a run

---

# M4 — Hybrid Local + Web Research Mode

## M4.1 Web policy integration
- [ ] Add web escalation policy to runtime config and execution context
- [ ] Enforce `off|ask|on` semantics
- [ ] Ensure child tasks inherit web policy correctly

### Acceptance
- [ ] Local-only runs never hit web tools unless allowed

## M4.2 External evidence normalization
- [ ] Define evidence record shape for web search/fetch results
- [ ] Separate local evidence from web evidence in trace
- [ ] Add citation formatter that labels local vs web sources

### Acceptance
- [ ] Final answers can distinguish local and external evidence

## M4.3 Escalation heuristics
- [ ] Define when local evidence is considered insufficient
- [ ] Add conservative heuristic for web escalation trigger
- [ ] Add tests for policy + trigger behavior

### Acceptance
- [ ] Web escalation only triggers when allowed and useful

---

# M5 — UX, Hardening, and Benchmarking

## M5.1 UX profiles
- [ ] Add profile resolution for `fast|balanced|deep|research`
- [ ] Document profile behavior and defaults
- [ ] Add tests for profile-to-config mapping

### Acceptance
- [ ] Users can start a session with one profile switch

## M5.2 Final answer formatting
- [ ] Add citation formatting helpers
- [ ] Add uncertainty/confidence note formatting
- [ ] Add trace id formatting for recursive runs

### Acceptance
- [ ] Final answers are grounded and distinguish evidence provenance

## M5.3 Telemetry and observability
- [ ] Add telemetry emissions for recursive lifecycle events
- [ ] Add counters for retrieval count, subcall count, web escalation count
- [ ] Ensure no sensitive blob dumping by default

### Acceptance
- [ ] Recursive runs are measurable and debuggable

## M5.4 Regression and fixture suite
- [ ] Add small fixture repo for codebase tasks
- [ ] Add fixture docs corpus for research tasks
- [ ] Add integration tests for retrieval and recursive flow
- [ ] Add benchmark-like smoke tests for large-ish corpora

### Acceptance
- [ ] MVP changes are protected by repeatable tests

---

# Stretch / Later Backlog
- [ ] Semantic retrieval backend
- [ ] Hybrid lexical + embedding ranking
- [ ] Smaller/cheaper default child-call models
- [ ] Parallel subqueries
- [ ] Browser automation layer
- [ ] Notebook/HTML visualizer for traces
- [ ] Adaptive confidence calibration
- [ ] AST-aware code retrieval

---

# Immediate Next Recommended Tasks

## Recommended next 5 tasks
- [ ] Run Rust workspace build/test audit
- [ ] Add typed `rag`/`rlm`/`webResearch` config structs
- [ ] Add corpus core structs and manifest storage scaffold
- [ ] Add trace ledger schema
- [ ] Add `CorpusSearch` MVP tool end-to-end

---

# Decision Log Hooks
Use this section to capture small decisions made during implementation.

- [ ] Decide v1 index storage format
- [ ] Decide default chunk size heuristic
- [ ] Decide child-call model default
- [ ] Decide trace artifact location
- [ ] Decide corpus attachment UX priority: config-first vs slash-command-first
