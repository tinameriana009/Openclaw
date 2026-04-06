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

### Child/provider execution
- cleaner shared provider child backend/factory direction
- less CLI-owned duplication
- better fallback messaging and backend availability reporting

### Local corpus RAG
- richer lexical ranking
- better retrieval explainability
- better slice diversity across documents
- skip telemetry / per-root reporting improvements

### Web-aware behavior
- child execution path now handles approved web evidence more honestly
- degraded web collection is surfaced as explicit notes instead of silent failure or overclaiming
- provenance handling is stronger

### Operator / product surface
- stronger root and Rust README docs
- bootstrap / quickstart guidance
- release/trust docs (`CHANGELOG`, `RELEASE`, `ARTIFACTS`, `PRIVACY`)
- domain workflow docs, prompt templates, and demo kits

### Showcase workflows
- Blender scene cleanup demo kit
- Unreal runtime telemetry demo kit
- Repo analysis demo kit

## Honest limitations
This repository is **not yet fully production-ready**.

Main reasons:
- web execution is still only minimally operational and not a fully mature end-to-end runtime path
- child execution is cleaner but still not perfectly centralized in the runtime layer
- retrieval is still primarily lexical
- recursive planning remains alpha, even though it is now much stronger than the baseline
- workflow validation for Blender/Unreal remains operator-driven rather than fully automated

## Recommended positioning
Use language like:

> Rust-first agent harness with real local corpus RAG, improving recursive runtime behavior, stronger traceability, and early hybrid local+web support. Suitable for serious local technical experimentation and grounded custom-task workflows, but not yet a fully production-ready universal agent platform.

## Suggested next focus
1. finalize release-candidate cleanliness and locked verification flow
2. continue moving child execution fully into shared runtime/provider abstractions
3. strengthen the minimal web executor into a more trustworthy path
4. improve retrieval quality beyond lexical-only limits
5. keep one showcase workflow (Blender) polished as the main proof path
