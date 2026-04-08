# Workflow recipe: Blender add-on creation

Use this when you want `claw` to help you design or implement a **Blender add-on** from a local workspace.

This recipe is honest about the current harness:

- it can inspect local code and docs
- it can keep a grounded working set via corpus attachment
- it can help plan files, operators, panels, preferences, and packaging layout
- it does **not** directly run Blender UI interactions for you
- success still depends on validating the generated Python inside Blender

## Best inputs

Attach one or more local roots such as:

- your add-on repo
- local Blender API notes or copied docs
- example add-ons you own or are allowed to inspect
- project notes, issue lists, or design docs
- the included demo kit at [`../examples/blender-scene-cleanup-demo/`](../examples/blender-scene-cleanup-demo/)

Example:

```bash
cd rust
./target/debug/claw --profile balanced \
  --corpus ../docs/examples/blender-scene-cleanup-demo \
  --corpus ../docs/prompts \
  prompt "Use the attached Blender demo corpus to explain the add-on layout, the user workflow, and the next implementation slice."
```

If you want a concrete starting point instead of a blank prompt, begin with:

- [`../examples/blender-scene-cleanup-demo/brief.md`](../examples/blender-scene-cleanup-demo/brief.md)
- [`../examples/blender-scene-cleanup-demo/validation-baseline.md`](../examples/blender-scene-cleanup-demo/validation-baseline.md)
- [`../examples/blender-scene-cleanup-demo/manual-test-checklist.md`](../examples/blender-scene-cleanup-demo/manual-test-checklist.md)
- [`../examples/blender-scene-cleanup-demo/addon/scene_cleanup_helper/`](../examples/blender-scene-cleanup-demo/addon/scene_cleanup_helper/)

## Recommended flow

### 1) Start with a planning brief

Use the template in [`../prompts/blender-addon-task.md`](../prompts/blender-addon-task.md).

Good first prompt shape:

```text
You are helping design a Blender add-on.
Ground your answer in the attached local corpus when possible.

Task:
Design a Blender add-on that [goal].

Constraints:
- Target Blender version: [version]
- Python-only unless stated otherwise
- Prefer minimal, understandable file layout
- Explicitly list operators, panels, property groups, and registration steps

Output:
1. Proposed package/file structure
2. Main user workflow
3. Key Blender API touchpoints
4. Risks or unknowns
5. A step-by-step implementation plan
```

### 2) Ask for the file map before code

Example:

```text
Based on the repository and corpus, propose the exact file tree and the responsibility of each file for this add-on. Keep it small.
```

### 3) Generate code in slices

Prefer targeted asks instead of “write the whole add-on”:

- `Create the registration and preferences skeleton.`
- `Draft the operator for batch renaming materials.`
- `Add a UI panel for scene-level controls.`
- `Refactor the property definitions into a dedicated module.`

This works better with traces and makes review easier.

### 4) Use traces for hard or surprising answers

When a response touches multiple files or reasoning steps, inspect the saved trace ledger:

```bash
ls .claw/trace
./target/debug/claw --resume latest /trace summary .claw/trace/<trace-file>
```

### 5) Run the lightweight coherence checks first

Before opening Blender, confirm the demo corpus is internally coherent:

```bash
python3 tests/validate_blender_demo.py
python3 docs/examples/blender-scene-cleanup-demo/tools/package_demo_addon.py
```

If you want a staged operator bundle instead of running those steps manually, use:

```bash
cd rust
./scripts/prepare-blender-demo.sh
```

That helper validates the demo kit, rebuilds the installable zip, and stages a review bundle under `.demo-artifacts/blender-demo/` with the checklist, baseline, operator findings template, follow-up prompt template, a bundle manifest, a machine-readable `bundle-summary.json`, and `bundle-checksums.txt` for repeatable handoff.

This still does **not** validate behavior in Blender, but it catches missing files, broken Python syntax, and packaging regressions early.

### 6) Validate in Blender manually against a baseline

Current harness gap: no built-in Blender execution loop.

After code generation, still do the real validation yourself:

- install the add-on in Blender
- confirm registration succeeds
- test operators against a disposable scene
- compare the reported counts with the baseline in [`../examples/blender-scene-cleanup-demo/validation-baseline.md`](../examples/blender-scene-cleanup-demo/validation-baseline.md)
- check panel visibility and property persistence
- verify packaging metadata and version compatibility

The repo includes a realistic manual checklist at [`../examples/blender-scene-cleanup-demo/manual-test-checklist.md`](../examples/blender-scene-cleanup-demo/manual-test-checklist.md).

### 7) Feed concrete observations back into the next prompt

Good follow-up evidence looks like:

- exact traceback text
- mismatched scan counts versus the baseline
- unclear panel wording
- Blender version-specific registration issues

That keeps the workflow grounded in observed behavior instead of generic "make it better" iteration.

If you used the staged bundle, start from `next-prompt-template.md` so the next operator or model turn inherits the exact Blender version, baseline mismatches, and traceback text instead of a memory-driven summary.

## Suggested corpus contents

For stronger grounded answers, include local material like:

- your current add-on source tree
- exported notes on `bpy.types.Operator`, `Panel`, `AddonPreferences`, `PropertyGroup`
- prior add-on examples
- QA notes from failed installs or traceback logs

## Good follow-up prompts

- `Compare two possible add-on layouts and recommend the simpler one.`
- `Explain which parts should be operators vs helper functions.`
- `Summarize all Blender API symbols used by this implementation.`
- `Review this traceback and propose the smallest fix.`
- `List the registration order dependencies in this package.`

## Example deliverables to ask for

- minimal package structure
- registration checklist
- test checklist for manual Blender validation
- release checklist for zip packaging
- migration notes between Blender API versions

## Current workflow limits

Be aware of the current product gaps:

- no dedicated Blender mode or schema-aware add-on generator
- no automatic Blender execution, reloading, or GUI testing
- no built-in packaging command specific to Blender add-ons
- grounding quality depends on the local docs/examples you attach
