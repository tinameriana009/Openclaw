# Workflow recipe: Unreal plugin creation

Use this when you want `claw` to help plan or draft an **Unreal Engine plugin** with a grounded local workspace.

This is a practical recipe for the current harness, not a claim of engine-native automation.

## What this workflow is good at

- understanding an existing Unreal repo or plugin layout
- summarizing local headers, module boundaries, and build files
- drafting plugin/module structure
- proposing `Build.cs`, `.uplugin`, and source file organization
- reviewing implementation plans and local code changes

## What it does not do today

- launch Unreal Editor for you
- compile or hot-reload inside the editor by itself
- validate reflection/macros beyond what exists in local source and docs
- replace real editor, build, or packaged-plugin validation

## Best inputs

Attach local corpus roots such as:

- the game repo or plugin repo
- copied engine integration notes
- existing plugins in your organization
- API notes or code examples you are allowed to use

Example:

```bash
cd rust
./target/debug/claw --profile deep \
  --corpus ../YourGame \
  --corpus ../docs \
  prompt "Plan an Unreal plugin for runtime telemetry export"
```

## Recommended flow

### 1) Ask for architecture first

Use the template in [`../prompts/unreal-plugin-task.md`](../prompts/unreal-plugin-task.md).

Good first prompt shape:

```text
You are helping create an Unreal Engine plugin.
Ground the answer in the attached local corpus.

Task:
Design a plugin that [goal].

Constraints:
- Unreal version: [version]
- Runtime, Editor, or both: [type]
- Prefer minimal module count
- Call out likely reflection/build risks explicitly

Output:
1. .uplugin sketch
2. Module layout
3. Public vs Private source tree plan
4. Build.cs dependencies to review
5. Step-by-step implementation plan
```

### 2) Ask for exact file creation list

Example:

```text
Given the attached repo, list the exact plugin files I should create or edit first, in dependency order.
```

### 3) Draft one module at a time

Safer asks:

- `Draft the .uplugin file and explain each field.`
- `Draft the Runtime module Build.cs with conservative dependencies.`
- `Create the main module class skeleton.`
- `Propose the public API header surface before implementation.`
- `Review whether this should be split into Runtime and Editor modules.`

### 4) Use repo analysis before large refactors

If the repo already contains Unreal code, ask grounded analysis questions before requesting edits:

- `Summarize existing plugin conventions in this repo.`
- `Find similar module startup patterns in the attached corpus.`
- `List existing Build.cs dependency patterns relevant to this plugin.`

### 5) Inspect trace artifacts when decisions matter

For multi-step recommendations:

```bash
ls .claw/trace
./target/debug/claw --resume latest /trace summary .claw/trace/<trace-file>
```

## Suggested corpus contents

For better answers, attach:

- plugin examples from your own codebase
- local engine wrapper code
- build errors or UnrealHeaderTool output logs
- internal conventions docs
- design notes describing runtime/editor boundaries

## Good follow-up prompts

- `Explain whether this feature belongs in a Runtime or Editor module.`
- `Review this Build.cs for likely unnecessary dependencies.`
- `Summarize all UObject-facing types I need for this feature.`
- `Turn this build error log into a probable root-cause checklist.`
- `Compare this plugin layout against patterns already present in the repo.`

## Example deliverables to ask for

- plugin file tree
- first-pass `.uplugin`
- first-pass `Build.cs`
- implementation sequence by file
- manual validation checklist for editor/build testing

## Current workflow limits

- no Unreal-aware structured generator mode
- no integrated editor launch / build / hot-reload loop
- no guarantee that generated macro/reflection code is build-clean without manual compile/test
- best results depend heavily on attaching your real repo and local plugin examples
