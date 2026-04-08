# Next Actions

## Goal
Move the repository from **disciplined alpha** toward a more genuinely production-ready local+recursive agent harness.

## Immediate Priorities

### 1. Preserve current status in docs
- Keep `README.md`, `rust/README.md`, `FINAL_STATUS.md`, `RELEASE_SUMMARY.md`, and `NEXT_ACTIONS.md` aligned
- Keep the newer planning docs (`PRODUCTION_READY_PRD.md`, `PRODUCTION_READY_CHECKLIST.md`, `READINESS_SCORECARD.md`, `IMPLEMENTATION_PLAN.md`) in sync with real behavior
- Avoid docs drift as new runtime work lands

### 2. Protect clean verification posture
- Keep the repo clean after salvage/integration work
- Remove or ignore local-only artifacts such as `.port_sessions/`
- Treat `rust/` as the real Rust workspace root for verification
- Reconfirm `cargo build --workspace --locked` and `cargo test --workspace --locked` whenever meaningful runtime work lands

### 3. Preserve fork-specific changelog clarity
- Maintain `FORK_CHANGELOG.md` as a concise explanation of what changed versus the earlier parity baseline
- Keep the README current-status block and fork-delta framing aligned

## Technical Priorities

### A. Make child execution runtime-native
Current state:
- provider-backed child execution exists and is materially better
- shared `api` helpers now own more model resolution, auth resolution, bounded minimal web-evidence shaping, shared recursive runtime construction, and more recursive query orchestration
- too much of the remaining behavior still lives in CLI-specific code

Next step:
- move child-provider execution behind a shared runtime/provider abstraction
- keep CLI as a client of that shared layer rather than the main home of the logic

Success criteria:
- child execution behavior becomes reusable across surfaces
- fallback behavior is policy-driven and consistently traceable
- runtime tests cover provider-backed and fallback modes directly

### B. Add real web executor integration
Current state:
- web policy/provenance/escalation handling is much better
- fetch-state-aware provenance, approval-required query preservation, operator-facing web execution summaries, resumed trace visibility for pending approval work, and degraded-path notes are now substantially more honest
- no full end-to-end web executor path is mature yet

Next step:
- add a minimal real web search/fetch executor path for recursive runs
- ensure `On`, `Ask`, and `Off` modes are consumed by execution logic, not only prompts/traces
- keep provenance explicit and safe

Success criteria:
- recursive runs can actually gather bounded external evidence when allowed
- ask-mode produces a trustworthy operator approval path
- final answers distinguish local and web evidence clearly

### C. Continue recursive runtime cleanup
Current state:
- `rlm.rs` has been modularized and stabilized significantly
- recursive behavior is much stronger than before, but still alpha

Next step:
- keep splitting responsibilities cleanly across `rlm/` submodules
- strengthen tests around:
  - stop reasons
  - partial failure paths
  - convergence/no-new-context behavior
  - token/cost/runtime boundary conditions

Success criteria:
- recursive engine is easier to maintain
- failure/stop behavior is easier to reason about
- regressions are harder to introduce

### D. Improve retrieval beyond lexical-only ceiling
Current state:
- local RAG works and is one of the strongest current features
- lexical retrieval will eventually limit quality on large/symbol-heavy tasks
- recent work added schemaVersion-aware artifact direction, chunk-neighbor retrieval, root-aware provenance, symbol-aware query heuristics, bounded semantic/language-intent signals, intent-routing/document-continuity signals, and outline-aware section routing as practical retrieval lifts

Next step:
- add better multi-corpus UX/reporting
- improve ranking explanations and skip telemetry
- add chunk-neighbor retrieval / sibling expansion for better context continuity
- investigate hybrid retrieval (lexical + semantic)
- eventually add symbol-aware or structure-aware retrieval for code workflows

Success criteria:
- better quality on large corpora
- less dependence on exact token overlap
- smoother repo analysis for complex codebases

## Workflow Priorities

### 1. Strengthen Blender workflow into a true alpha demo
Current state:
- repo analysis is now the best current runnable end-to-end showcase path
- Blender still remains the strongest domain-specific alpha demo
- Blender and Unreal now also have honest prep helpers for repeatable operator handoff
- docs, prompt templates, example brief, and demo kit exist

Next step:
- create a tighter end-to-end operator walkthrough
- add clearer expectations around how to validate the generated add-on manually
- optionally add one more illustrative Blender example

Success criteria:
- a new operator can follow one Blender workflow path without guessing
- repo has one convincing domain workflow showcase

### 2. Keep Unreal workflow honest and useful
Current state:
- Unreal workflow docs exist, and the operator handoff/error-feedback loop is now better structured
- the workflow is still assistive rather than smooth

Next step:
- keep scope narrow and honest
- document best practices for using local docs/examples corpus with Unreal
- avoid overclaiming automation

Success criteria:
- users understand where the harness helps and where it still struggles

## Release / Trust Priorities

### 1. Add artifact schema/version metadata
Current state:
- artifact trust docs exist
- trace/corpus artifacts are documented but not explicitly schema-versioned

Next step:
- add explicit schema/version fields to trace and corpus artifacts
- document compatibility expectations

Success criteria:
- artifact evolution becomes safer
- future migrations are easier to reason about

### 2. Improve release discipline
Current state:
- changelog/release docs exist
- RC readiness is more formalized and release artifact manifests now exist
- no full release automation yet

Next step:
- adopt a simple release routine
- keep `rust/CHANGELOG.md` updated
- use `rust/RELEASE.md` checklist for each meaningful release cut

Success criteria:
- releases feel deliberate and reproducible

## Suggested Execution Order

### Short term
1. Push latest stable local work
2. Reconfirm green build/test on pushed state
3. Move child execution toward shared runtime/provider abstraction
4. Add minimal real web executor path

### Medium term
5. Add artifact schema/version metadata
6. Improve retrieval quality and multi-corpus UX
7. Strengthen Blender workflow demo further
8. Tighten recursive runtime tests and stop/failure semantics

### Longer term
9. Hybrid retrieval / semantic support
10. Better code-aware retrieval for large/symbol-heavy tasks
11. Richer approval UX and provenance views
12. More mature domain workflows beyond Blender

## Practical KPI Checklist
The repo is moving closer to production-ready when these become true consistently:
- `cargo build --workspace` green in CI
- `cargo test --workspace` green in CI
- first-run docs can be followed by a new operator
- local custom tasks succeed with low friction
- child provider execution is stable
- local+web provenance is trustworthy
- at least one domain workflow succeeds end-to-end convincingly

## Honest Current Constraint
Do not overclaim maturity while these remain true:
- web retrieval is still only partially operational end-to-end
- recursive planning is still alpha
- retrieval is still mostly lexical
- some domain workflows remain operator-heavy
