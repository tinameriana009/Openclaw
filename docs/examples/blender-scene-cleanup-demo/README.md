# Blender scene cleanup demo kit

This directory is a **tiny local corpus** for one concrete Blender workflow: a scene-cleanup add-on that helps an artist prepare a file before export.

It is meant to make the current harness feel more real without pretending it can run Blender for you.

## What is included

- `brief.md` — a realistic operator brief you can paste into `claw`
- `manual-test-checklist.md` — the human validation loop to run inside Blender
- `validation-baseline.md` — a sample disposable scene recipe plus the counts you should expect after a scan
- `addon/scene_cleanup_helper/` — a minimal illustrative add-on package you can inspect, compare against, or ask the model to extend
- `tools/package_demo_addon.py` — a tiny helper that zips the demo add-on for Blender installation tests

## What this kit is for

- grounding prompts in a small but coherent Blender-specific corpus
- giving operators a believable first workflow to try end-to-end
- showing the kind of package layout, operators, properties, and UI split that the harness can discuss usefully
- giving humans a concrete validation target instead of a vague "test it in Blender" instruction

## What this kit is not

- a guaranteed production-ready Blender add-on
- an automated Blender test harness
- proof that the Rust CLI can drive Blender UI actions directly
- a substitute for validating behavior inside a real Blender 4.x install

## Recommended operator path

From the `rust/` directory:

```bash
./target/debug/claw --profile balanced \
  --corpus ../docs/examples/blender-scene-cleanup-demo \
  --corpus ../docs/prompts \
  prompt "Use the attached Blender demo corpus to explain the add-on layout, the user workflow, and the next implementation slice."
```

Then continue with narrower prompts such as:

- `List the operators, properties, and panel responsibilities in this demo add-on.`
- `Propose the smallest patch to add a report for unapplied transforms.`
- `Turn this demo into a release checklist for a real Blender 4.x add-on.`
- `Review the code for Blender API assumptions and call out anything that still needs manual validation.`

## Best demo sequence for operators

1. Read `brief.md` to frame the task.
2. Read `validation-baseline.md` so you know what a correct scan should roughly report.
3. Run `python3 tests/validate_blender_demo.py` to confirm the local kit is intact.
4. Optionally build an installable zip with `python3 docs/examples/blender-scene-cleanup-demo/tools/package_demo_addon.py`.
5. If you want a cleaner handoff bundle, run `cd rust && ./scripts/prepare-blender-demo.sh` to stage artifacts under `.demo-artifacts/blender-demo/`.
6. Install the zipped or copied add-on in Blender.
7. Follow `manual-test-checklist.md` against a disposable scene.
8. Feed any traceback, confusing UI wording, or mismatched counts back into the next prompt.

That sequence gives you a more convincing first-run story: corpus → plan → static validation → installable artifact → staged operator bundle → manual Blender validation.

## Lightweight local validation

This repo includes a small verification script:

```bash
python3 tests/validate_blender_demo.py
```

That script only checks **static coherence**:

- required demo files exist
- the illustrative Python package parses/compiles
- the packaging helper emits a zip artifact
- key workflow docs mention the demo kit and manual validation reality

It does **not** launch Blender.

## Expected demo outcome

If you follow the disposable scene recipe in `validation-baseline.md`, the current add-on should report:

- duplicate materials: **1**
- unapplied transforms: **2** when hidden objects are excluded
- unapplied transforms: **3** when hidden objects are included

Those numbers are small on purpose: they give operators something concrete to confirm without pretending the demo covers full production cleanup.
