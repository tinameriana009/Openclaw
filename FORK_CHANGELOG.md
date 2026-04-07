# Fork Changelog

This file summarizes the most meaningful changes in `tinameriana009/Openclaw` compared with the earlier parity baseline / upstream reference point.

## Current Position
This fork is no longer just a parity snapshot.

Current honest label:
- **strong alpha / early pre-production candidate**

Current strongest areas:
- local corpus RAG
- recursive runtime hardening
- trace export and telemetry
- operator/release/trust docs
- demo/workflow packaging for Blender, repo analysis, and Unreal

## Major Areas of Change

### 1. Local corpus RAG became a real product surface
Compared with the earlier baseline, this fork now has much more practical corpus support:
- local corpus attach/persist flows
- inspect/search/slice flows
- richer lexical ranking
- retrieval explainability improvements
- slice diversity improvements
- root-aware retrieval provenance
- multi-root disambiguation for same-named files

Why it matters:
- the harness is much more useful for grounded repo/docs work
- citations and evidence are more trustworthy in multi-corpus scenarios

### 2. Recursive runtime moved beyond a thin scaffold
This fork substantially strengthened recursive/iterative execution:
- bounded recursive flow
- stronger stop semantics
- better partial-failure handling
- modularization under `rust/crates/runtime/src/rlm/`
- cleaner finalization behavior
- stronger stop-event trace metadata

Why it matters:
- recursive runs are more debuggable and less fragile
- the runtime is closer to a maintainable long-term architecture

### 3. Child execution became more shared and less CLI-owned
The fork moved key child-execution assembly out of CLI-local wiring and into shared layers:
- shared provider extractive child executor builder in `api`
- less duplication in corpus-answer child execution paths
- cleaner direction toward runtime/provider-owned behavior

Why it matters:
- architecture is cleaner
- non-CLI surfaces have a better path to reuse the same behavior later

### 4. Web-aware behavior became more honest and observable
The baseline had earlier scaffolding, but this fork improved how web-aware paths are represented:
- explicit web execution outcome handling
- better degraded-state notes
- richer trace counters and telemetry for web execution
- explicit web-execution completion events in traces
- more honest final-answer summaries when web approval/evidence is partial or absent

Why it matters:
- less overclaiming
- better operator trust
- easier debugging of hybrid local+web flows

### 5. Trace and telemetry became much more useful
This fork improved operator visibility across the harness:
- structured trace ledger work
- improved counters and summaries
- stronger evidence/provenance rendering
- richer recursive/web stop/finalization data

Why it matters:
- the system is easier to inspect and reason about
- operator confidence is higher during complex flows

### 6. Release / trust / operator posture improved significantly
This fork now includes much stronger operational docs than the baseline:
- `rust/README.md`
- `rust/BOOTSTRAP.md`
- `rust/CHANGELOG.md`
- `rust/RELEASE.md`
- `rust/docs/ARTIFACTS.md`
- `rust/docs/PRIVACY.md`
- `rust/docs/COMPATIBILITY.md`
- `rust/docs/REDACTION.md`
- readiness and workflow validators in `tests/`

Why it matters:
- the repo is easier to operate honestly
- release posture is more deliberate
- demo/workflow docs are held to a stronger honesty bar

### 7. Workflow demos became much more concrete
Compared with the baseline, the fork now includes stronger practical demo material:
- Blender demo kit
- repo-analysis demo kit
- Unreal demo kit
- workflow docs and prompt templates
- lightweight demo/readiness validators

Why it matters:
- the repo can show realistic usage stories instead of only abstract capability claims

## Notable Recent Batch
A recent integration batch added or confirmed:
- production-readiness planning docs
- implementation plan docs
- readiness scorecard/checklist docs
- release/workflow readiness validators
- web execution tracing semantics improvements
- recursive stop trace hardening
- shared provider child executor refactor
- retrieval provenance improvements
- salvage sync for `web_execution` runtime surface/docs

## What Still Has Not Changed
This fork is still **not fully production-ready**.

Main remaining limits:
- web execution is still not a fully mature end-to-end operator path
- child execution is improved but not yet fully runtime/provider-owned
- retrieval is still primarily lexical
- recursive planning remains alpha rather than fully mature orchestration
- some workflows remain operator-heavy even when the docs are much better

## Suggested README Framing
A good short description of the fork is:

> Rust-first agent harness fork with real local corpus RAG, improving recursive runtime behavior, stronger traceability, more honest hybrid local+web scaffolding, and better release/operator workflow discipline than the earlier parity baseline.
