# Remaining Gaps v3

This file is the final synced snapshot after Option B, `gaps-v9`, retrieval stabilization, runtime-interface integration fixes, web approval lifecycle polish, provenance witness work, and final locked verification from `rust/`.

## Current Overall Position
Current honest label:
- **very near-production candidate**

Current honest readiness:
- **~97.5%**

Verified from the real workspace root:
- `~/.cargo/bin/cargo build --workspace --locked` ✅
- `~/.cargo/bin/cargo test --workspace --locked` ✅

## What is now settled
The repository now has all of the following in a materially real form:
- local corpus attach/search/slice workflows
- bounded recursive runtime / RLM behavior
- richer retrieval heuristics with structure-aware and semantic-ish lifts
- stronger traceability and telemetry
- bounded local+web approval/review/resume flows
- stronger release/trust artifacts, including signed provenance and bounded external witness support
- improved shared runtime/provider direction
- repeatable operator handoff helpers for domain demos

This is no longer an “early serious pre-production” repo. It is much closer to the finish line than that.

## Remaining Gaps

### 1. Web lifecycle is still bounded, not live-interactive
What is now strong:
- `/trace approve`, `/trace review`, `/trace replay`, `/trace resume`, `/trace approvals`
- static HTML/Markdown/JSON review surfaces
- explicit operator commands embedded in artifacts
- approval-required query preservation and clearer next-step guidance

What is still missing:
- no real browser automation
- no click-to-rerun dashboard behavior
- no live session/web UI lifecycle
- no truly seamless operator-in-the-loop web workflow

Why it still matters:
- this is still the easiest place to overclaim
- the repo now has a useful bounded review surface, but not a real interactive web product loop

### 2. Runtime/provider abstraction is much better, but not fully universal
What is now strong:
- shared provider runtime client moved into the `api` layer
- shared provider-backed recursive/runtime setup is materially stronger
- less CLI-only orchestration glue than before

What is still missing:
- CLI still has rendering/progress-specific behavior that is not cleanly generalized
- some provider-configured child/runtime seams still live in specialized codepaths
- the runtime/provider abstraction is improved, not fully finished

Why it still matters:
- future entrypoints will still benefit from one more cleanup pass
- architectural finish is one of the last major quality multipliers left

### 3. Retrieval is strong, but still heuristic-first
What is now strong:
- symbol-aware retrieval
- outline/section-aware routing
- neighbor expansion
- evidence-set reranking
- cross-document agreement
- morphology-aware normalization
- bounded semantic expansion for common repo/docs vocabulary
- stabilized regression coverage around the richer retrieval stack

What is still missing:
- no embedding-backed semantic retrieval
- no AST/LSP-aware structural retrieval
- no deeper learned reranking stage
- no broad retrieval-strategy planner across retrieval modes

Why it still matters:
- large and structure-heavy corpora will still eventually hit heuristic ceilings
- this is now more of a quality ceiling than a correctness blocker

### 4. Provenance is stronger, but still not public supply-chain-grade
What is now strong:
- artifact manifests
- attestation and signed provenance
- trust policy
- bounded rooted X.509 mode
- bounded external/publication witness layer

What is still missing:
- no transparency log
- no keyless identity flow
- no hosted builder attestation
- no Sigstore/SLSA-grade public provenance story
- no remote witness replay/verification path

Why it still matters:
- current trust is meaningful and useful, but still source/operator-anchored
- public supply-chain posture is still one of the last missing “finish-line” layers

### 5. Recursive engine is credible, but not planner-grade
What is now strong:
- bounded recursive execution
- better stop/convergence behavior
- better child failure handling
- improved breadth bias toward unseen docs
- stronger planner/progress metadata in traces

What is still missing:
- no fully adaptive planner policy
- no mature retry/escalation strategy
- no richer query diversification planner
- no production-grade orchestrator maturity

Why it still matters:
- the engine is now very respectable for a bounded recursive system
- but difficult multi-step tasks can still expose the gap to a true planner-grade orchestrator

### 6. Domain workflows are still operator-heavy
What is now strong:
- repo analysis is a good runnable showcase
- Blender/Unreal prep and handoff flows are more repeatable
- combined domain bundle staging exists

What is still missing:
- no real Blender/Unreal editor automation
- no runtime validation inside those external apps
- no smooth packaged product experience for domain workflows

Why it still matters:
- these workflows are now honest and usable, but still not fully automated

## Updated Priority Order
1. live-interactive web lifecycle
2. final runtime/provider abstraction cleanup
3. deeper semantic/structure-aware retrieval beyond heuristics
4. public provenance finalization
5. planner-grade recursive maturity
6. richer domain-app automation

## Honest Final Read
This repository is now:
- **highly usable for serious local technical work**
- **strong enough for grounded corpus-backed and bounded recursive workflows**
- **much closer to production than alpha framing would imply**

But it is still not honestly:
- fully production-ready
- a live web product
- a planner-grade orchestrator
- a public supply-chain-grade release system
- a true app automation platform

## Recommendation
If the goal is to stop on a stable, honest, high-quality checkpoint, this is a good stopping point.

If the goal is to keep pushing, the top two multipliers are still:
1. web lifecycle maturity
2. universal runtime/provider cleanup
