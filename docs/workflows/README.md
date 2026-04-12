# Domain workflows and task recipes

These are **operator recipes** for the current Rust harness.

They are meant to make `claw` feel more product-like for real work without overstating what the code does today.

## What these recipes assume

Current, verified building blocks:

- `claw prompt ...` and `claw "..."` for one-shot work
- interactive REPL via `claw`
- execution profiles: `fast`, `balanced`, `deep`, `research`
- local corpus attachment via `--corpus` and `/corpus attach`
- grounded retrieval and recursive answer path via `/corpus answer ...`
- trace inspection via `/trace ...` and saved ledgers in `.claw/trace/`
- session autosave / resume via `.claw/sessions/`

## What these recipes are not

- not a one-click workflow engine
- not a Blender or Unreal generator plugin
- not a replacement for domain SDK docs, engine docs, or build systems
- not a guarantee that the model will produce working code on the first try

They are best used as:

1. a repeatable prompt scaffold
2. a grounded local-analysis loop
3. a traceable review flow for multi-step answers

Always run the lightweight validators before trusting a showcase, then finish with the real human validation loop for the target tool or engine.

## Recommended baseline loop

For any substantial task:

1. attach the repo or docs you care about with `--corpus` or `/corpus attach`
2. pick a profile:
   - `balanced` for normal work
   - `deep` for harder local analysis
   - `research` when you want the largest local investigation budget
3. ask for a plan first
4. ask for concrete patches, file lists, or implementation steps second
5. force a validation loop before you trust the answer:
   - ask what specific build/test/manual check should happen next
   - ask what evidence actually changed between iterations
   - treat repeated answers without new evidence as convergence, not progress
6. inspect `.claw/trace/` if the reasoning path matters
7. resume the session instead of restarting from scratch

## Available recipes

- [Blender add-on creation](blender-addon.md)
- [Unreal plugin creation](unreal-plugin.md)
- [General repository analysis](repo-analysis.md)

## Prompt templates and example briefs

Prompt templates live in [`../prompts/`](../prompts/).

Starter example briefs live in [`../examples/`](../examples/).

If you want the best current end-to-end showcase path, start with the repo analysis demo kit and its helper script in `rust/scripts/run-repo-analysis-demo.sh`. It stays closest to what the harness can genuinely do today: grounded local analysis, session resume, trace review, static handoff/dashboard artifacts, an explicit `operator-transition-brief.md` baton pass, and a `continuity-status.json` ledger without pretending to drive an external GUI.

If you want one especially concrete operator path, start with one of these demo kits:

### Blender scene cleanup demo kit

- [`../examples/blender-scene-cleanup-demo/README.md`](../examples/blender-scene-cleanup-demo/README.md)
- [`../examples/blender-scene-cleanup-demo/brief.md`](../examples/blender-scene-cleanup-demo/brief.md)
- [`../examples/blender-scene-cleanup-demo/manual-test-checklist.md`](../examples/blender-scene-cleanup-demo/manual-test-checklist.md)
- [`../examples/blender-scene-cleanup-demo/next-prompt-template.md`](../examples/blender-scene-cleanup-demo/next-prompt-template.md)
- prep helper: `../../rust/scripts/prepare-blender-demo.sh`
- combined bundle helper: `../../rust/scripts/prepare-domain-demo-bundles.sh`

### Repo analysis demo kit

- [`../examples/repo-analysis-demo/README.md`](../examples/repo-analysis-demo/README.md)
- [`../examples/repo-analysis-demo/brief.md`](../examples/repo-analysis-demo/brief.md)
- [`../examples/repo-analysis-demo/manual-validation-checklist.md`](../examples/repo-analysis-demo/manual-validation-checklist.md)
- [`../examples/repo-analysis-demo/operator-session-template.md`](../examples/repo-analysis-demo/operator-session-template.md)
- [`../examples/repo-analysis-demo/next-prompt-template.md`](../examples/repo-analysis-demo/next-prompt-template.md)
- [`../examples/repo-analysis-demo/trace-review-checklist.md`](../examples/repo-analysis-demo/trace-review-checklist.md)
- staged continuity artifacts: `.demo-artifacts/repo-analysis-demo/<timestamp>/{operator-dashboard.html,bundle-summary.json,operator-handoff.json,review-status.json,continuity-status.json,runtime-bridge.json,runtime-events.jsonl,operator-transition-brief.md,review-log.md,bundle-checksums.txt}`
- cross-run static review index: `.demo-artifacts/repo-analysis-demo/{index.json,index.html}`

### Unreal runtime telemetry demo kit

- [`../examples/unreal-runtime-telemetry-demo/README.md`](../examples/unreal-runtime-telemetry-demo/README.md)
- [`../examples/unreal-runtime-telemetry-demo/brief.md`](../examples/unreal-runtime-telemetry-demo/brief.md)
- [`../examples/unreal-runtime-telemetry-demo/manual-validation-checklist.md`](../examples/unreal-runtime-telemetry-demo/manual-validation-checklist.md)
- [`../examples/unreal-runtime-telemetry-demo/error-feedback-playbook.md`](../examples/unreal-runtime-telemetry-demo/error-feedback-playbook.md)
- [`../examples/unreal-runtime-telemetry-demo/operator-session-template.md`](../examples/unreal-runtime-telemetry-demo/operator-session-template.md)
- [`../examples/unreal-runtime-telemetry-demo/next-prompt-template.md`](../examples/unreal-runtime-telemetry-demo/next-prompt-template.md)
- [`../examples/unreal-runtime-telemetry-demo/trace-review-checklist.md`](../examples/unreal-runtime-telemetry-demo/trace-review-checklist.md)
- prep helper: `../../rust/scripts/prepare-unreal-demo.sh`
- combined bundle helper: `../../rust/scripts/prepare-domain-demo-bundles.sh`

## Quick validation commands

Run these from the repository root to confirm the showcase assets and release-facing workflow docs are still honest and wired correctly:

```bash
python3 tests/validate_operator_readiness.py
python3 tests/validate_blender_demo.py
python3 tests/validate_unreal_demo.py
python3 tests/validate_repo_analysis_demo.py
```

These checks only validate documentation/demo coherence. They do not replace Blender, Unreal, or repo-specific manual verification.

If you want a single prep pass for the two app-domain demos, run `cd rust && ./scripts/prepare-domain-demo-bundles.sh`. It stages both bundles together for operator handoff, but still stops short of editor/app automation.
