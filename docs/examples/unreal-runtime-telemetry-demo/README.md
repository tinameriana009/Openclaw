# Unreal runtime telemetry demo kit

This demo kit gives the Unreal plugin workflow a more believable first-run path without pretending the harness can drive Unreal Editor, UnrealBuildTool, or Live Coding for you.

It gives operators:

- a concrete local corpus for one plausible plugin
- a realistic brief to paste into `claw`
- an expected-findings sheet to compare against
- a manual validation checklist for the real editor/build loop
- an error-feedback playbook for turning real Unreal failures into better follow-up prompts
- an operator session template for capturing environment, logs, and runtime observations
- a trace-review checklist for auditing how the answer was assembled
- a tiny illustrative plugin skeleton you can inspect before asking for changes

## What is included

- `brief.md` — a grounded task brief for a runtime telemetry plugin
- `expected-findings.md` — facts the model should usually recover from the demo corpus
- `manual-validation-checklist.md` — the operator loop for build/editor/runtime validation
- `error-feedback-playbook.md` — prompt recipes for Build.cs, UHT, load, Blueprint, and runtime failures
- `operator-session-template.md` — a lightweight worksheet for recording environment, logs, and next-prompt payloads
- `trace-review-checklist.md` — how to review the saved reasoning trail when an answer feels suspicious
- `plugin/RuntimeTelemetry/` — a minimal illustrative Unreal plugin layout with `.uplugin`, `Build.cs`, module bootstrapping, a subsystem, and a Blueprint-facing library

## What this kit is for

- grounding Unreal prompts in an actual plugin-shaped corpus
- making the current workflow feel more operationally honest
- teaching operators to separate static planning from real compile/editor validation
- giving humans a concrete checklist instead of a vague “test it in Unreal” instruction

## What this kit is not

- a production-ready Unreal plugin
- proof that the harness can run Unreal builds or editor automation
- a substitute for compiling with your actual engine/toolchain
- a guarantee that generated reflection macros or module dependencies are correct for your environment

## Recommended operator path

From `rust/` after building `claw`:

```bash
./target/debug/claw --profile balanced \
  --corpus ../docs/examples/unreal-runtime-telemetry-demo \
  --corpus ../docs/prompts \
  prompt "Use the attached Unreal demo corpus to explain the plugin layout, the likely operator workflow, and the next implementation slice."
```

Then continue with narrower asks such as:

- `List the plugin files in dependency order and explain why they exist.`
- `Review the Build.cs dependencies and call out anything risky or unnecessary.`
- `Explain what should stay in the subsystem versus the Blueprint library.`
- `Turn the manual validation checklist into a release-readiness checklist for a real project.`
- `Review this UnrealBuildTool or compiler error and propose the smallest likely fix.`

## Best demo sequence for operators

1. Read `brief.md` to frame the task.
2. Run `python3 tests/validate_unreal_demo.py` to confirm the local demo kit is intact.
3. Compare the plugin skeleton against `expected-findings.md` so you know what a grounded answer should recover.
4. Use `claw` with the demo corpus attached to ask for the architecture or next implementation slice.
5. If you want a cleaner handoff bundle, run `cd rust && ./scripts/prepare-unreal-demo.sh` to stage artifacts under `.demo-artifacts/unreal-demo/`.
6. Use `operator-session-template.md` while validating so environment details, logs, and runtime observations do not get lost.
7. Copy the plugin into a disposable Unreal project or compare it against an existing plugin repo.
8. Follow `manual-validation-checklist.md` for the real compile/editor/runtime loop.
9. Use `error-feedback-playbook.md` to turn compiler errors, UHT issues, load failures, or Blueprint surprises into the next grounded prompt.

That path is intentionally honest: corpus → plan → static validation → staged operator bundle → human build/editor validation → evidence-driven follow-up.

## Lightweight local validation

This repo includes a small coherence check:

```bash
python3 tests/validate_unreal_demo.py
```

That script only checks static facts:

- required demo files exist
- the `.uplugin` JSON parses
- the C#-style `Build.cs` and core C++ files contain expected Unreal-specific anchors
- the workflow docs point operators to the demo kit and the manual validation loop

It does **not** launch Unreal Editor, run UnrealBuildTool, or compile the plugin.
