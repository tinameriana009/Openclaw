# PRD — Production Readiness Push

## Status
Draft v0.1

## Purpose
This document defines the next product phase required to move `Openclaw` from a **strong alpha / early pre-production candidate** into a more defensible production-ready state.

It does **not** assume the project becomes a universal agent platform immediately. Instead, it focuses on:
- stabilizing the current strengths
- closing the most important architectural gaps
- improving trust, repeatability, and operator confidence
- proving one or two workflows end-to-end

## Current State
The project already has:
- a real Rust-first harness
- local corpus RAG that works in practice
- recursive runtime behavior that is substantially more real than a scaffold
- trace export and telemetry
- provider-backed child execution paths
- web-aware policy/provenance support
- workflow docs and demo kits for Blender, Unreal, and repo analysis
- release/trust/operator docs

## Main Problem
The repo is useful and increasingly disciplined, but it is not yet comfortably production-ready because several key surfaces remain incomplete or not fully centralized:
- child execution is not fully runtime-native
- web execution is still only minimally operational
- retrieval quality remains lexical-heavy
- recursive behavior is still alpha rather than fully dependable production orchestration
- workflow demos are stronger than before, but still mostly operator-driven rather than end-to-end reliable
- release/trust discipline is good, but not yet final

## Product Goal
Move the project to a state where it can be honestly positioned as:

> a production-capable local-first agent harness for grounded technical tasks, with dependable recursive behavior, strong traceability, and bounded hybrid local+web support

## Non-Goals
- full autonomous software factory behavior
- universal domain-specific automation for all IDE/game/creative workflows
- replacing all operator judgment
- pretending semantic retrieval or web execution is mature before it is

## Success Criteria
We should consider this phase successful when:
1. full build/test/verification is stable and repeatable
2. child execution is centrally owned by shared runtime/provider abstractions
3. web execution has a trustworthy bounded path with explicit degraded/approval semantics
4. retrieval quality is visibly stronger on realistic corpora
5. one showcase workflow (Blender) is convincing enough to demo end-to-end
6. docs, trust notes, release posture, and status reporting remain synced to actual behavior

## Workstreams

### Workstream 1 — Release Candidate Discipline
Objective:
- make the repo feel like a controlled release candidate rather than a moving lab bench

Key outcomes:
- locked build/test stable
- release verification helper used and documented
- remote/release posture unambiguous
- docs reflect exact expected release flow

### Workstream 2 — Runtime-Native Child Execution
Objective:
- make child execution policy and backend construction belong to shared runtime/provider abstractions, not mostly CLI wiring

Key outcomes:
- reduced CLI ownership
- clearer shared backend/factory contract
- runtime-facing child execution policy becomes easier to reason about

### Workstream 3 — Web Executor Maturity
Objective:
- move from “web-aware” to “bounded, real, honest web behavior”

Key outcomes:
- richer web execution state
- stronger degraded-state handling
- better provenance/tracing
- approval semantics clearer and more trustworthy

### Workstream 4 — Retrieval Quality Uplift
Objective:
- reduce the practical ceiling of lexical-only retrieval

Key outcomes:
- better ranking quality
- better explainability
- better multi-corpus clarity
- improved context continuity and selection behavior

### Workstream 5 — Workflow Realism
Objective:
- turn one or two workflow demos into convincing operator stories

Key outcomes:
- Blender workflow becomes a serious alpha showcase
- repo analysis workflow becomes more repeatable
- Unreal workflow remains honest but better structured

## Prioritized Engineering Focus
1. runtime-native child execution finalization
2. web executor maturity
3. retrieval quality beyond lexical-only ceiling
4. release candidate final polish
5. Blender workflow end-to-end operator confidence

## Risks
1. runtime complexity keeps drifting back into CLI code
2. web mode gets overclaimed before it is truly mature
3. docs drift away from the actual runtime again
4. retrieval quality improvements stall before semantic/hybrid approaches are ready
5. workflows look good in docs but still do not feel convincing in practice

## Mitigations
- keep tests close to runtime behavior changes
- preserve honest wording in docs/help
- prefer practical bounded improvements over giant speculative rewrites
- keep status docs (`FINAL_STATUS.md`, `NEXT_ACTIONS.md`, `RELEASE_SUMMARY.md`) updated
- validate showcase workflows through explicit manual/testable checklists

## Recommended Next Checkpoint
The next good checkpoint after this PRD should be:
- refreshed status docs
- updated scorecard
- green locked verification
- one additional measurable improvement in child execution, web maturity, or retrieval quality
