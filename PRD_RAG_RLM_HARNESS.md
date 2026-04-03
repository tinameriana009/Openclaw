# PRD — RAG + Recursive Language Model (RLM) Integration for Claw Harness

## Status
Draft v0.1

## Context
We want to evolve `claw-code-parity` from a strong standalone harness into a practical long-context agent runtime that can:
- operate as a primary harness for new sessions
- use repository/document retrieval when useful (RAG)
- use **Recursive Language Models** as described by Alex Zhang to reason over large context without stuffing all context into a single model call
- support browser/web-aware research workflows where local context is insufficient

This PRD assumes the current repository already provides:
- a CLI/runtime surface in `rust/`
- built-in tools such as file read/write/edit, grep/glob, web search/fetch, todo/task tracking, and sub-agent launching
- session persistence, permissions, and prompt assembly

## Problem Statement
Modern harnesses degrade when the session or context becomes large. Even with long context windows, quality often drops due to context rot, poor salience selection, and bloated chat history. Basic retrieval helps, but standard RAG alone is not sufficient for:
- multi-hop reasoning over many documents
- tasks requiring adaptive decomposition
- long-running coding/research sessions where the model must inspect, summarize, refine, and recurse

We need a system where the model can:
1. inspect very large context indirectly
2. decide how to partition it
3. recursively issue sub-queries against slices of context
4. aggregate and refine intermediate results
5. stop when confidence is sufficient or budget is exhausted

## Definitions

### RAG
Retrieval-Augmented Generation: fetch only the most relevant chunks/documents before or during reasoning.

### RLM
For this project, **Recursive Language Model** means a scaffold around a base LM where the model can recursively call sub-model/sub-runtime queries over transformed subsets of context, using an environment such as a REPL or structured controller loop.

### Browser/Web Check
Any workflow where the agent determines local context is insufficient and uses web fetch/search or browser-capable tools to gather external evidence.

## Vision
A user should be able to start a new harness session and ask questions over:
- a codebase
- a local document corpus
- a large note/archive store
- optionally the web

The harness should answer by combining:
- direct tool use
- targeted retrieval
- recursive decomposition over context
- explicit review/refinement cycles

The user experience should still feel like a single agent call, even though the system may perform many sub-steps underneath.

## Goals
1. **Session-ready harness**: make the repo usable as the runtime for new agent sessions.
2. **Integrated RAG**: support local corpus indexing + retrieval over repos/docs/notes.
3. **Integrated RLM orchestration**: allow the root model to recursively query subsets of context via a controlled loop.
4. **Web escalation**: allow the system to use web search/fetch when local evidence is insufficient.
5. **Operational safety**: bound recursion by time, cost, depth, and token budget.
6. **Observability**: make recursive traces inspectable so users can understand what happened.
7. **Good default UX**: simple commands/config for starting sessions with RAG+RLM enabled.

## Non-Goals
- Training a new foundation model
- Reproducing the exact MIT RLM research stack or benchmark results
- Infinite recursion or fully unconstrained self-spawning agents
- Full browser automation in v1 unless needed for critical workflows
- Perfect automatic retrieval over every modality from day one

## Primary Users
1. **Solo technical operator**
   - wants a powerful coding/research harness over local repos and notes
2. **Research-heavy user**
   - wants to query huge local corpora and optionally the web
3. **Power user of agent workflows**
   - wants recursion, inspection, memory, and decomposition rather than one-shot prompting

## User Stories

### US1 — Codebase analysis
As a user, I want to ask a question over a large repo and have the harness retrieve relevant files, recurse over candidate modules, and return a grounded answer.

### US2 — Massive local corpus search
As a user, I want to load many documents and still get strong answers without manually chunking or pruning context.

### US3 — Multi-hop research
As a user, I want the harness to combine local retrieval with web evidence when the answer is not fully contained in the local corpus.

### US4 — Inspectability
As a user, I want to inspect the recursive trace: what the root model searched, what sub-queries it issued, and why it stopped.

### US5 — Safe execution
As an operator, I want recursion budgets, max depth, and max iterations so the system does not spiral.

## Product Requirements

## 1. Session Model
The harness must support starting a new session with configurable:
- model/provider
- permission mode
- workspace root
- RAG source set(s)
- RLM mode on/off
- recursion depth and budget limits
- web escalation policy

### Acceptance Criteria
- User can start a session with a single command or config profile.
- Session metadata records whether RAG and RLM were enabled.
- Session resume preserves RLM/RAG settings.

## 2. Retrieval (RAG) Layer
The system must support retrieval over:
- repository files
- markdown/docs/notes
- optionally exported artifacts or logs

### v1 Retrieval Modes
1. **Lexical retrieval**
   - `grep`, `glob`, filename/path heuristics
2. **Structured manifest retrieval**
   - file inventory, summaries, extension/type metadata
3. **Optional semantic retrieval**
   - embedding/vector index as a later phase

### Required Behaviors
- chunk documents into retrieval units
- attach metadata: file path, headings, language, size, modified time, chunk id
- support top-k retrieval with score and source attribution
- support dynamic retrieval inside a recursive loop

### Acceptance Criteria
- User can index a local corpus.
- User can run a retrieval query and see cited chunk/file results.
- Runtime can call retrieval during task execution.

## 3. Recursive Language Model (RLM) Controller
The root runtime must support a recursive control loop where the model can:
- inspect corpus size/shape without loading all content directly
- peek at subsets of context
- search/filter/partition context
- spawn sub-queries against selected context slices
- summarize or extract intermediate findings
- aggregate results into a final answer

### Core RLM Concepts for v1
- **Root controller**: main model handling the user task
- **Sub-call engine**: isolated sub-query execution against a narrowed context
- **Context object**: corpus is treated as an inspectable object, not blindly pasted prompt text
- **Trace ledger**: every recursive step logged as structured state

### v1 Design Constraints
- default max recursion depth = 1 or 2
- default max iterations per request
- bounded token/cost/time budget
- deterministic stop conditions where possible

### Candidate Controller Operations
- `peek_context`
- `search_context`
- `slice_context`
- `summarize_slice`
- `subquery_context`
- `aggregate_findings`
- `finalize_answer`

### Acceptance Criteria
- Root session can issue at least one recursive sub-query over a context slice.
- Sub-query result is returned as structured output to the parent.
- Parent can iterate over multiple slices before finalization.
- Execution stops safely at configured limits.

## 4. Web / Browser Escalation
When local corpus confidence is low or evidence is incomplete, the harness should be able to escalate to web-backed tools.

### v1 Scope
- web search
- web fetch
- source citation
- policy gate for when external access is allowed

### v2+ Scope
- browser automation / page interaction if truly needed

### Acceptance Criteria
- Runtime can explicitly mark an answer as requiring external evidence.
- Web-derived evidence is stored separately from local retrieval results.
- Final answer labels which evidence came from local corpus vs web.

## 5. Explainability and Traceability
Each RLM request should optionally produce a trace artifact that captures:
- task id/session id
- retrieval calls
- selected slices
- sub-query prompts/roles at a summarized level
- sub-query outputs
- aggregation steps
- stop reason
- budget consumed

### Acceptance Criteria
- Trace can be emitted to file/JSON.
- User can inspect high-level trace without exposing raw private corpus unless permitted.
- Failure modes are visible (budget exceeded, no relevant evidence, recursion cap hit).

## 6. Configuration
Configuration should support profiles like:
- `coding-rag`
- `research-rag-web`
- `rlm-local-corpus`
- `rlm-hybrid`

### Config Needs
- provider/model for root and sub-calls
- optional smaller model for sub-calls
- retrieval backend choice
- max documents/chunks per recursion step
- recursion depth
- iteration cap
- time budget
- cost budget
- external web allowed: yes/no/ask

## 7. Safety and Control
The system must avoid uncontrolled recursion and unsafe external actions.

### Controls
- recursion depth limit
- max subcalls
- total request timeout
- per-step timeout
- token/cost cap
- permission boundary inheritance to subcalls
- external access policy
- write/exec restrictions preserved under recursion

### Acceptance Criteria
- Child calls cannot exceed parent permission scope.
- Runtime aborts cleanly when budget or cap is exceeded.
- Trace records abort reason.

## Technical Architecture

## A. High-Level Components
1. **CLI / Session Runtime**
2. **Tool Registry**
3. **Retrieval Engine**
4. **RLM Controller**
5. **Sub-call Execution Engine**
6. **Trace / Ledger Store**
7. **Optional Web Evidence Layer**

## B. Proposed Runtime Flow
1. User submits task.
2. Runtime classifies task type and decides whether RAG/RLM is needed.
3. Retrieval engine builds an initial context map.
4. Root model receives:
   - the task
   - summary of available context
   - tools for peeking/searching/slicing/subquerying
5. Root model performs iterative reasoning:
   - inspect
   - search
   - recurse
   - aggregate
6. Runtime validates stop conditions.
7. Final answer returned with sources and optional trace.

## C. Suggested Code Placement
Given current repo shape, a likely Rust implementation split is:
- `rust/crates/runtime/` — RLM session controller, budgets, recursion policy
- `rust/crates/tools/` — retrieval and context inspection tools
- `rust/crates/commands/` — session/config/status commands for RAG/RLM
- `rust/crates/api/` — provider clients and model routing for root/sub-calls
- new crate or module for corpus indexing if separation becomes useful

## Functional Design Details

## 1. Corpus Abstraction
Represent local context as a corpus object rather than raw text blob.

Example fields:
- corpus id
- roots
- document count
- chunk count
- size estimate
- chunk metadata
- retrieval backend metadata

## 2. Sub-query Contract
Each recursive sub-call should receive:
- parent task id
- sub-task description
- narrowed context slice ids
- allowed tools subset
- budget subset
- expected output schema

Each result should return:
- answer text or structured extraction
- confidence estimate if available
- cited chunks/documents
- cost/timing metadata
- stop reason

## 3. Retrieval Strategy
### Phase 1
- path-aware lexical retrieval
- grep/glob based narrowing
- heading-aware markdown slicing
- code symbol/file summary lookup

### Phase 2
- embeddings/vector index
- hybrid ranking
- query reformulation for sub-queries

## 4. Stop Conditions
Stop recursion when any of the following holds:
- answer confidence sufficient
- evidence converges
- no better slices found
- iteration cap reached
- depth cap reached
- budget exhausted
- user requested fast mode

## 5. Output Contract
Final answers should support:
- concise final answer
- cited sources
- confidence / uncertainty note
- optional trace id
- optional recommendations for deeper follow-up

## Success Metrics

### Product Metrics
- time to useful answer on large corpora
- answer quality vs current non-RLM baseline
- user-visible citation quality
- percentage of tasks solved without manual context stuffing

### Runtime Metrics
- average recursive depth
- average sub-call count
- retrieval hit rate
- web escalation rate
- timeout/cap abort rate
- cost per resolved task

## Phased Delivery Plan

## Phase 0 — Audit and Scaffold
- build current repo
- identify session/runtime insertion points
- define config schema for RAG/RLM
- decide trace format

**Deliverable:** architecture note + implementation skeleton

## Phase 1 — Local RAG MVP
- corpus indexing for files/docs
- lexical retrieval API
- source citation formatting
- commands to inspect corpus/index

**Deliverable:** local corpus retrieval inside harness

## Phase 2 — RLM MVP
- root controller loop
- context peek/search/slice tools
- depth-1 sub-query execution
- trace ledger and budget controls

**Deliverable:** first working recursive reasoning path over local corpus

## Phase 3 — Hybrid RAG + RLM
- dynamic retrieval inside recursive loop
- aggregation over sub-results
- better stop policies
- benchmark on real repo/doc tasks

**Deliverable:** usable long-context coding/research workflow

## Phase 4 — Web-Aware Research Mode
- escalation from local to web evidence
- evidence separation and provenance
- confidence/risk labeling

**Deliverable:** hybrid local+web research harness

## Phase 5 — Advanced Enhancements
- semantic retrieval
- smaller/cheaper sub-call models
- parallel sub-calls
- richer browser integrations
- notebook/visual trace UI

## Open Questions
1. Should sub-calls use the same provider/model as the root, or cheaper/faster models by default?
2. Do we want strict tree recursion, or a general task graph with recursion-like semantics?
3. Should retrieval be exposed as tools only, or partly embedded in controller policy?
4. How much of the recursive trace should be shown to end users by default?
5. Should web escalation be automatic, ask-first, or profile-based?
6. Do we want RLM over only text, or eventually code ASTs / structured corpora too?

## Risks
- uncontrolled cost/time from recursion
- retrieval quality not good enough to guide recursion
- too much flexibility causing unstable agent behavior
- prompt/trace bloat from verbose recursive logging
- confusing UX if users cannot tell when recursion helps vs hurts

## Mitigations
- conservative defaults
- explicit budgets and caps
- trace summarization rather than raw transcript dumping
- benchmark against baseline harness behavior
- add fast/balanced/deep execution profiles

## Initial Recommendation
For this repository, the best first implementation path is:
1. build and validate the current Rust harness
2. add a **local lexical RAG MVP** first
3. layer an **RLM controller with depth-1 subqueries** on top of that
4. only then add hybrid local+web escalation

This keeps the first version grounded in the repo’s existing strengths: tool calling, session state, search/fetch tools, and sub-agent-style orchestration.

## Proposed Artifacts After PRD Approval
- architecture spec
- config schema draft
- task breakdown/implementation plan
- minimal benchmark suite for large-context repo/doc tasks
