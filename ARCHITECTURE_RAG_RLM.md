# Architecture Spec — RAG + Recursive Language Model Harness

## Status
Draft v0.1

## Relationship to PRD
This document operationalizes `PRD_RAG_RLM_HARNESS.md` into a technical architecture for the current `claw-code-parity` repository.

Where the PRD defines product direction, this document defines:
- runtime boundaries
- module placement
- data models
- execution flow
- configuration surfaces
- implementation sequence

## Existing Repository Baseline
The Rust workspace already provides a strong harness foundation:

- `rust/crates/runtime/`
  - session state
  - config loading
  - permission policy
  - conversation loop
  - hooks, compaction, sandbox, usage, OAuth
- `rust/crates/tools/`
  - built-in tool registry
  - file and web tools
  - todo + agent-like tooling
- `rust/crates/api/`
  - provider clients and streaming
  - prompt cache support
- `rust/crates/rusty-claude-cli/`
  - user-facing CLI / REPL entrypoint
- `rust/crates/telemetry/`
  - session tracing
- `rust/crates/plugins/`
  - plugin-backed tool extension surface

Important current anchors:
- `runtime/src/conversation.rs`
  - current root turn loop
- `runtime/src/config.rs`
  - typed config loading and precedence
- `tools/src/lib.rs`
  - tool registry, tool definitions, tool execution

## Architectural Goal
Extend the current harness into a **session-ready coding/research runtime** with:
1. a **local retrieval layer**
2. a **recursive reasoning controller**
3. a **trace/ledger model** for recursion and evidence
4. **hybrid local+web evidence orchestration**

The architecture should preserve the current harness strengths:
- one-session conversational UX
- explicit tool use
- permission inheritance
- session persistence
- traceability

## Design Principles

### 1. Harness-first, not retriever-first
RAG and RLM are added as capabilities inside the harness. They should not feel like detached side systems.

### 2. Corpus as object, not pasted blob
Large context should be represented as a corpus abstraction that can be inspected, searched, sliced, and cited.

### 3. Conservative recursion by default
Depth, time, and cost are bounded from day one.

### 4. Retrieval before recursion when cheap
Prefer low-cost narrowing before issuing sub-calls.

### 5. Explainability over magic
Every recursive execution path should produce inspectable structured state.

### 6. Keep v1 mostly lexical and local
Do not block on vector search, browser automation, or large parallel execution.

## High-Level Runtime Model

```text
User Task
   |
   v
CLI / Session Runtime
   |
   +--> Config + Permissions + Session State
   |
   +--> Retrieval Planner
   |       |
   |       +--> Corpus Index
   |       +--> Lexical Search
   |       +--> Manifest / Chunk Metadata
   |
   +--> RLM Controller
   |       |
   |       +--> Context Peek/Search/Slice
   |       +--> Sub-query Execution
   |       +--> Aggregation / Stop Policy
   |
   +--> Web Escalation Layer
   |       |
   |       +--> WebSearch
   |       +--> WebFetch
   |
   +--> Answer / Action Synthesizer
           |
           +--> Final Answer + Citations + Trace Summary
```

## Proposed Module Layout

## 1. `rust/crates/runtime/`
Primary home for orchestration policy and recursive control.

### New modules proposed
- `runtime/src/rag.rs`
  - runtime-facing retrieval orchestration
  - initial retrieval planning
  - source selection policies
- `runtime/src/rlm.rs`
  - recursive controller
  - iteration loop
  - stop criteria
  - parent/child budgets
- `runtime/src/trace.rs`
  - RLM trace ledger structs + serialization helpers
- `runtime/src/corpus.rs`
  - corpus/session corpus attachment abstraction
- `runtime/src/budget.rs`
  - time/cost/depth/iteration accounting

### Existing modules to extend
- `runtime/src/config.rs`
  - parse RAG/RLM config sections
- `runtime/src/conversation.rs`
  - expose integration points for recursive mode
- `runtime/src/session.rs`
  - persist session-level RAG/RLM metadata if needed

## 2. `rust/crates/tools/`
Primary home for context-inspection and retrieval tools.

### New tool families proposed
- `CorpusAttach`
- `CorpusInspect`
- `CorpusSearch`
- `CorpusSlice`
- `CorpusSummarize`
- `RlmSubquery`
- optional later: `CorpusIndexStatus`

### Existing tools reused
- `read_file`
- `glob_search`
- `grep_search`
- `WebSearch`
- `WebFetch`
- `TodoWrite`
- `Agent` (optional future leverage)

## 3. `rust/crates/api/`
Provider/model routing for root and child calls.

### Likely extensions
- root model vs sub-call model selection
- child-call request helpers
- child-call cost metadata exposure

## 4. `rust/crates/rusty-claude-cli/`
User-facing entrypoint.

### Likely extensions
- CLI flags / config profile selection for RAG/RLM
- slash commands for corpus status / trace inspection
- optional trace export UX

## Core Concepts

## 1. Corpus
A corpus is a structured representation of local context sources attached to a session.

### Corpus fields
```text
Corpus {
  id
  roots[]
  kind (repo|docs|notes|mixed)
  document_count
  chunk_count
  estimated_bytes
  backend (lexical|hybrid|semantic)
  documents[]
}
```

### Document fields
```text
Document {
  document_id
  path
  media_type
  language
  headings[]
  bytes
  modified_at
  chunks[]
}
```

### Chunk fields
```text
Chunk {
  chunk_id
  document_id
  ordinal
  start_offset
  end_offset
  text_preview
  metadata
}
```

## 2. Retrieval Result
```text
RetrievalResult {
  query
  backend
  hits[]
  elapsed_ms
}

RetrievalHit {
  chunk_id
  document_id
  path
  score
  reason
  preview
}
```

## 3. RLM Task Context
Represents one recursive reasoning request.

```text
RlmTask {
  task_id
  parent_task_id?
  session_id
  user_goal
  working_query
  allowed_corpus_ids[]
  allowed_tools[]
  recursion_depth
  budget
  state
}
```

## 4. Budget Model
```text
Budget {
  max_depth
  max_iterations
  max_subcalls
  max_runtime_ms
  max_prompt_tokens
  max_completion_tokens
  max_cost_usd?
}
```

Child calls inherit a reduced budget slice from the parent.

## 5. Trace Ledger
A trace is a structured event stream, not raw chain-of-thought.

```text
Trace {
  trace_id
  session_id
  root_task_id
  started_at
  finished_at?
  events[]
  final_status
}
```

### Event types
- task_started
- retrieval_requested
- retrieval_completed
- corpus_peeked
- corpus_sliced
- subquery_started
- subquery_completed
- web_escalation_started
- web_evidence_added
- aggregation_completed
- stop_condition_reached
- task_failed

## Execution Architecture

## A. Root Turn Flow

### Standard path
1. User submits prompt.
2. CLI/runtime resolves config.
3. Harness chooses execution mode:
   - direct tool loop
   - RAG-assisted
   - RLM-assisted
   - hybrid local+web
4. Runtime builds a `RootExecutionPlan`.
5. Controller executes plan.
6. Final answer + citations + trace summary returned.

### Proposed execution mode selector
Simple v1 heuristic:
- direct mode for short/simple tasks
- RAG mode when task references files/docs/corpus
- RLM mode when:
  - corpus is large
  - task is multi-hop
  - retrieval confidence is low
  - explicit deep mode enabled
- web escalation when local evidence is insufficient and policy allows

## B. RAG Flow
1. Session has one or more attached corpora.
2. Retrieval planner transforms user query into search query/queries.
3. Tools/backends gather top-k lexical matches.
4. Runtime creates a compact context map.
5. Root model receives only the compact map plus retrieval tools.

## C. RLM Flow
1. Root controller sees task + corpus summary.
2. Root controller may:
   - inspect corpus stats
   - peek selected docs/chunks
   - issue searches
   - create slices
   - launch subqueries
3. Subqueries run on narrowed context only.
4. Parent collects structured outputs.
5. Parent aggregates and decides whether to stop or continue.

## D. Web Escalation Flow
1. Controller determines evidence gap.
2. If policy allows, `WebSearch` / `WebFetch` is invoked.
3. External evidence is normalized into separate trace/evidence records.
4. Parent task combines local and web evidence.

## Integration Strategy with Existing Runtime

## 1. Keep the current `ConversationRuntime` intact for baseline loop behavior
Current `ConversationRuntime` already gives:
- assistant <-> tool loop
- permission checks
- hooks
- iteration cap
- usage tracking
- session tracing

### Architectural choice
Do **not** replace `ConversationRuntime` immediately.
Instead, add a higher-level orchestration layer that can:
- prepare system prompt / tool availability differently
- attach corpus metadata
- optionally wrap or re-enter child runtime calls

### Proposed new wrapper
```text
RecursiveConversationRuntime
  - wraps ConversationRuntime
  - manages corpus + retrieval + subcalls
  - emits trace ledger
```

This can live in `runtime/src/rlm.rs` and use `ConversationRuntime` as a lower-level building block.

## 2. Extend config instead of inventing a second config system
`runtime/src/config.rs` already parses structured config.

### Proposed new config sections
```json
{
  "rag": {
    "enabled": true,
    "backend": "lexical",
    "defaultCorpora": ["."] ,
    "chunkBytes": 2000,
    "maxHits": 12
  },
  "rlm": {
    "enabled": true,
    "maxDepth": 1,
    "maxIterations": 8,
    "maxSubcalls": 6,
    "maxRuntimeMs": 45000,
    "subcallModel": "haiku",
    "trace": true
  },
  "webResearch": {
    "mode": "ask",
    "maxFetches": 5
  }
}
```

### Parsing additions needed
- typed `RuntimeRagConfig`
- typed `RuntimeRlmConfig`
- typed `RuntimeWebResearchConfig`
- integration into `RuntimeFeatureConfig`

## 3. Add tools before adding hidden controller-only logic where possible
v1 should favor tool-shaped affordances so behavior remains inspectable.

### Proposed tool contract examples

#### `CorpusInspect`
Input:
- corpus id
- limit
- optional path prefix

Output:
- corpus stats
- matching docs/chunks

#### `CorpusSearch`
Input:
- corpus id
- query
- top_k
- path filters optional

Output:
- scored hits with preview + source

#### `CorpusSlice`
Input:
- corpus id
- chunk ids or path/offset range

Output:
- narrowed slice payload + metadata

#### `RlmSubquery`
Input:
- parent task id
- subquery text
- selected chunk ids
- optional output schema

Output:
- structured answer
- cited sources
- stop reason / confidence / usage

## 4. Telemetry and trace should reuse `telemetry` crate semantics where possible
Do not create a completely separate logging worldview.

### Proposed split
- telemetry crate: event emission plumbing
- trace module: RLM-specific domain records and serialization

## Configuration and Control Surfaces

## User-facing modes
- `fast`
  - direct tool loop, minimal retrieval
- `balanced`
  - lexical retrieval, shallow recursion disabled unless needed
- `deep`
  - lexical retrieval + RLM depth-1 enabled
- `research`
  - local retrieval + optional web escalation

## CLI additions proposed
- `--rag on|off`
- `--rlm on|off`
- `--corpus <path>` repeated
- `--depth <n>`
- `--trace`
- `--web on|off|ask`
- `--profile fast|balanced|deep|research`

## Slash commands proposed
- `/corpus`
- `/corpus attach <path>`
- `/corpus search <query>`
- `/trace`
- `/trace export [path]`
- `/mode [fast|balanced|deep|research]`

## Security and Safety Model

## Permission inheritance
Child/subquery executions must not exceed parent permissions.

### Rule
`child_permission_scope <= parent_permission_scope`

## External access control
Web escalation obeys explicit config/session policy:
- `off`
- `ask`
- `on`

## Recursion safety
Enforced in controller, not only prompt text:
- depth cap
- subcall cap
- iteration cap
- timeout cap
- optional cost cap

## Trace privacy
Trace should capture decisions and evidence references, but avoid dumping entire private corpora unless explicitly exported.

## Detailed Component Design

## 1. Corpus Indexer
### Responsibilities
- crawl configured roots
- classify files
- chunk textual content
- build manifest/index
- support refresh/rebuild

### v1 backend
- manifest + lexical search over text chunks
- likely file-backed JSON/JSONL or lightweight SQLite later

### Storage options
#### v1 recommendation
- manifest JSON + chunk JSONL under harness data dir

Reason:
- fast to implement
- inspectable
- minimal external dependencies

## 2. Retrieval Planner
### Responsibilities
- accept task/query
- decide retrieval scope
- run lexical narrowing
- return compact evidence map

### v1 ranking inputs
- path/file name match
- heading match
- grep hit count
- extension/type priors
- recency/size penalties if useful

## 3. RLM Controller
### Responsibilities
- run root recursive loop
- expose corpus operations to model
- spawn child calls
- maintain budget and trace
- stop safely

### Internal state proposal
```text
RlmControllerState {
  current_iteration
  retrieval_history
  selected_slices
  child_results
  evidence_pool
  stop_reason?
}
```

## 4. Subquery Executor
### Responsibilities
- create isolated child request
- pass narrowed evidence only
- optionally use smaller model
- return structured result

### v1 note
Child calls can be implemented as ordinary provider calls or mini runtime invocations with reduced tool surface.

### Recommended v1
Use **ordinary model/provider child calls** with strict schema and no arbitrary tool expansion.

Why:
- easier to implement
- lower risk than full nested agent runtime
- enough to prove recursive value

## 5. Answer Synthesizer
### Responsibilities
- combine direct reasoning + retrieval + child outputs
- format citations
- surface uncertainty
- attach trace id

## Failure Modes and Recovery

### Failure modes
- no corpus attached
- retrieval returns weak evidence
- recursion cap reached
- child call timeout
- web escalation denied
- malformed trace state

### Recovery expectations
- degrade gracefully to direct answer when safe
- explicitly state uncertainty
- record stop reason in trace

## Test Strategy

## Unit tests
- config parsing for rag/rlm/web sections
- corpus chunking
- lexical ranking
- budget enforcement
- child permission inheritance
- trace event serialization

## Integration tests
- repo corpus attach + search
- root task with depth-1 subquery
- local-only task vs local+web task
- stop behavior on depth/time cap

## Fixture style
Prefer tiny synthetic corpora:
- small code repo
- markdown notes corpus
- multi-doc research set

## Recommended First Increment
To keep scope sane, first engineering increment should deliver:
1. typed config additions
2. corpus manifest/index MVP
3. corpus search tool MVP
4. trace ledger schema
5. shallow RLM controller with one child subquery path

That is enough to prove the architecture without solving every later optimization.

## Explicit Deferrals
Not required for first working architecture:
- embeddings/vector DB
- browser automation
- parallel subqueries
- DAG recursion
- learning-based planner
- automatic confidence calibration

## Open Technical Decisions
1. child calls as plain provider calls vs mini nested runtime
2. file-backed index vs SQLite in v1
3. whether corpus operations are all tools or partly internal APIs
4. how much trace data is shown inline by default
5. whether session persistence should store corpus snapshots or references only

## Recommended Answers for v1
1. plain provider child calls
2. file-backed manifest/index
3. mostly tool-shaped corpus ops, controller policy internal
4. inline trace summary + explicit export for details
5. store corpus references + derived metadata, not full duplicate corpus data
