# Blender scene cleanup demo kit

This directory is a **tiny local corpus** for one concrete Blender workflow: a scene-cleanup add-on that helps an artist prepare a file before export.

It is meant to make the current harness feel more real without pretending it can run Blender for you.

What is included:

- `brief.md` — a realistic operator brief you can paste into `claw`
- `manual-test-checklist.md` — the human validation loop to run inside Blender
- `addon/scene_cleanup_helper/` — a minimal illustrative add-on package you can inspect, compare against, or ask the model to extend

What this kit is **for**:

- grounding prompts in a small but coherent Blender-specific corpus
- giving operators a believable first workflow to try end-to-end
- showing the kind of package layout, operators, properties, and UI split that the harness can discuss usefully

What this kit is **not**:

- a guaranteed production-ready Blender add-on
- an automated Blender test harness
- proof that the Rust CLI can drive Blender UI actions directly

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

## Lightweight local validation

This repo includes a small verification script:

```bash
python3 tests/validate_blender_demo.py
```

That script only checks **static coherence**:

- required demo files exist
- the illustrative Python package parses/compiles
- key workflow docs mention the demo kit

It does **not** launch Blender.
