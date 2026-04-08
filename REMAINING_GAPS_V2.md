# Remaining Gaps v2

This file summarizes the most meaningful remaining gaps after the latest weak-area integration batch (including release artifact manifest work, Unreal workflow handoff improvements, richer operator-facing web summaries, deeper retrieval heuristics, and more shared child orchestration).

## Current Overall Position
Current honest label:
- **strong alpha / early serious pre-production candidate**

The project is now materially stronger than the earlier parity baseline and has passed recent locked workspace verification from `rust/` with the pinned toolchain. However, a few important gaps still prevent an honest production-ready label.

## 1. Web maturity is still not end-to-end
What is now strong:
- fetch-state-aware web provenance
- approval-required query preservation
- degraded-path reporting
- richer final-answer web summaries
- operator-facing web execution status / next-step guidance
- stronger web trace counters and summaries

What is still missing:
- no true interactive approval-resume loop
- no mature operator browser/session workflow
- no dedicated web-run dashboard or rich trace inspection UX
- bounded hybrid local+web behavior still stops short of a production-grade web path

Why it still matters:
- this remains the easiest area to overclaim
- trust depends on honest execution semantics and clear operator visibility

## 2. Child orchestration is not fully generalized yet
What is now strong:
- shared provider child executor setup
- shared auth resolution
- shared bounded web adapter path
- shared recursive runtime builder
- shared provider-backed recursive query orchestration for corpus answer flows

What is still missing:
- the shared runner is still corpus-answer specific rather than a more general recursive task orchestration surface
- CLI still owns some final presentation/rendering behavior
- orchestration primitives are improved, but not yet distilled into one broad reusable provider/runtime abstraction for future entrypoints

Why it still matters:
- future surfaces will be easier to build if orchestration is truly shared
- architectural cleanliness is one of the biggest remaining quality multipliers

## 3. Retrieval is still heuristic-first rather than truly semantic
What is now strong:
- root-aware provenance
- symbol-aware matching
- language-intent-aware scoring
- query-intent routing
- section-aware retrieval signals
- document continuity signals
- bounded semantic expansion for common repo/docs vocabulary
- better explainability and reason strings

What is still missing:
- no embedding-based semantic retrieval
- no AST/LSP-aware retrieval for code structure
- no document-outline / heading hierarchy model
- no stronger reranking stage over retrieved evidence sets
- no explicit query planner choosing retrieval strategy classes

Why it still matters:
- heuristic retrieval can still plateau on large or structure-heavy corpora
- this will eventually become the main answer-quality ceiling for more advanced custom tasks

## 4. Release artifact trust is still not formal provenance
What is now strong:
- RC flow is more formalized
- release validators are better
- release artifact manifest exists
- key trust/release surfaces are hashed and machine-readable

What is still missing:
- no signed provenance or attestation
- no packaged release artifact trust chain
- no automatic fresh-run artifact verification pipeline end-to-end
- trust is still mostly local/source-first rather than formal release provenance

Why it still matters:
- stronger release discipline improves confidence for reuse and handoff
- this is important if the repo is going to be presented as more than an advanced source-first harness

## 5. Workflow realism still depends heavily on operators
What is now strong:
- repo analysis is the best current runnable showcase path
- Blender workflow has a more honest prep and validation story
- Unreal workflow now has a better error-feedback and operator handoff loop

What is still missing:
- Blender and Unreal still depend on real manual validation in external tools/editors
- no editor/app automation loop for those domains
- no smooth end-to-end domain workflow comparable to a packaged product experience

Why it still matters:
- realistic workflow credibility matters more than demo breadth
- domain workflows are where users notice operational friction fastest

## 6. Recursive engine maturity is still alpha
What is now strong:
- bounded recursive behavior
- better stop/failure semantics
- stronger traceability
- cleaner modular structure than before
- better handling of partial failures and web-aware child paths

What is still missing:
- no mature planner/orchestrator
- no advanced adaptive recursive policy layer
- limited production-grade stress handling for complex recursive task classes
- still more of a serious alpha engine than a fully dependable orchestrator

Why it still matters:
- recursive quality affects hard multi-step tasks more than simple corpus lookups
- this is one of the last large maturity jumps before “production-ready” becomes plausible

## Updated Priority Order
1. web maturity end-to-end
2. generalized child orchestration
3. deeper semantic / structure-aware retrieval beyond heuristics
4. release artifact provenance / trust finalization
5. workflow realism for Blender / Unreal
6. recursive engine maturity

## Honest Current Readiness Read
Approximate current readiness:
- **~81%**

This is best described as:
- **early serious pre-production**

Not yet:
- fully production-ready
- fully mature for arbitrary web-heavy or domain-app-heavy tasks

## Suggested Next Batch
A sensible next batch would focus only on the top 3 gaps:
1. web maturity end-to-end
2. generalized child orchestration
3. deeper semantic retrieval

Those three are the biggest remaining technical multipliers.
