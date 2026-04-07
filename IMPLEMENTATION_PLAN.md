# Implementation Plan

## Purpose
This document turns the production-readiness planning docs into an execution-oriented plan.

Related docs:
- `PRODUCTION_READY_PRD.md`
- `PRODUCTION_READY_CHECKLIST.md`
- `READINESS_SCORECARD.md`
- `FINAL_STATUS.md`
- `NEXT_ACTIONS.md`

## Current State
The repository is currently best described as:
- **strong alpha / early pre-production candidate**
- strongest areas: local corpus RAG, traceability, operator docs, improving recursive runtime
- weakest areas: fully mature web execution, fully runtime-native child execution, retrieval beyond lexical ceiling, fully convincing end-to-end workflows

## Execution Strategy
We should avoid giant rewrites.

Preferred strategy:
1. keep the repo green
2. land bounded improvements in one area at a time
3. keep docs and release posture synced to behavior
4. use one showcase workflow (Blender) as the main proof path

## Workstreams

### Workstream A — Release Candidate Discipline
**Goal:** make the repo easier to verify, release, and trust.

#### Tasks
- [ ] ensure all new release/readiness docs are linked from README surfaces appropriately
- [ ] verify `rust/scripts/release-verify.sh` matches the true release gate
- [ ] reduce remaining release-friction issues in docs and helper scripts
- [ ] verify local vs remote posture before each release push
- [ ] define a lightweight repeatable release routine using `rust/CHANGELOG.md` + `rust/RELEASE.md`

#### Done when
- a maintainer can run one documented flow and know whether the repo is release-candidate clean

---

### Workstream B — Runtime-Native Child Execution
**Goal:** centralize child execution behavior into shared runtime/provider abstractions.

#### Tasks
- [ ] continue moving provider child backend construction behind shared factory/builder paths
- [ ] reduce remaining CLI-owned child execution policy
- [ ] centralize availability/unavailable/fallback semantics
- [ ] standardize child output metadata and fallback reasoning
- [ ] add direct tests for shared child backend behavior, not only CLI behavior

#### Done when
- child execution policy is mostly runtime/provider-owned and CLI is only a consumer

---

### Workstream C — Web Executor Maturity
**Goal:** make web behavior bounded, honest, and more operationally trustworthy.

#### Tasks
- [ ] formalize web execution state beyond loose notes where practical
- [ ] improve degraded-state tracing and operator visibility
- [ ] improve approval/ask-mode semantics without pretending full interactive approval exists where it does not
- [ ] keep web provenance explicit in final answers and trace output
- [ ] add tests covering approved, denied, degraded, and local-only paths

#### Done when
- web-enabled runs are traceable and honest enough that operators can trust what happened

---

### Workstream D — Retrieval Quality Uplift
**Goal:** raise retrieval quality without a giant architecture jump.

#### Tasks
- [ ] add chunk-neighbor / sibling expansion for better context continuity
- [ ] improve multi-corpus reporting and selection clarity
- [ ] improve identifier-aware matching or query normalization where practical
- [ ] keep ranking explainability visible to operators
- [ ] continue regression coverage on retrieval quality behavior

#### Done when
- retrieval feels more robust on realistic repos/docs and is easier to debug

---

### Workstream E — Recursive Runtime Hardening
**Goal:** make the recursive engine more dependable.

#### Tasks
- [ ] continue cleaning the modular `rlm/` structure
- [ ] expand tests for stop semantics, degraded paths, and failure boundaries
- [ ] improve convergence/no-new-context handling where practical
- [ ] improve output/trace consistency when child execution partially fails or degrades
- [ ] avoid unnecessary planner overreach; prioritize dependable bounded behavior

#### Done when
- recursive behavior feels stable enough for repeated practical use, not only demos

---

### Workstream F — Workflow Realism
**Goal:** make showcase workflows feel more believable and useful.

#### Tasks
- [ ] keep Blender as the main showcase workflow
- [ ] improve Blender operator loop and validation guidance further
- [ ] keep Unreal docs honest while making them more concrete where possible
- [ ] keep repo-analysis workflow repeatable and reviewable
- [ ] add lightweight validation assets/tests when they improve operator confidence

#### Done when
- at least one workflow is convincing enough to serve as a showcase in README/docs

## Suggested Order of Execution

### Phase 1 — Stabilize and centralize
1. Workstream B — Runtime-Native Child Execution
2. Workstream C — Web Executor Maturity
3. Workstream A — Release Candidate Discipline

### Phase 2 — Raise quality
4. Workstream D — Retrieval Quality Uplift
5. Workstream E — Recursive Runtime Hardening

### Phase 3 — Prove usability
6. Workstream F — Workflow Realism

## Milestone Targets

### Milestone 1
- shared child execution cleaner
- web path more honest and traceable
- release verification flow reliable

### Milestone 2
- retrieval quality improved further
- recursive engine harder to break
- docs still synced

### Milestone 3
- Blender workflow becomes the strongest public demonstration path

## Verification Expectations
Every meaningful milestone should, at minimum, re-check:
- `cargo build --workspace --locked`
- `cargo test --workspace --locked`
- any targeted workflow/demo validators added by the milestone

## Success Markers
We are getting close to production-ready when:
- workspace verification remains green consistently
- child execution is no longer obviously CLI-owned
- web behavior is bounded, explicit, and trustworthy
- retrieval quality noticeably improves on realistic corpora
- Blender workflow feels like a real operator story, not just a doc exercise
- release and trust docs remain aligned to actual repo behavior

## Anti-Goals During Implementation
- do not overclaim web maturity before it exists
- do not chase a giant planner rewrite unless absolutely necessary
- do not let docs drift behind code again
- do not sacrifice green verification for flashy but unstable features
