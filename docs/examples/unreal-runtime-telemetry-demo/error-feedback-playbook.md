# Error feedback playbook: Unreal runtime telemetry demo

Use this after a real Unreal build/editor attempt. The goal is to turn vague "it failed" reports into evidence that the next `claw` prompt can actually use.

## Operating rule

Always feed back **exact evidence**:

- copied compiler errors
- copied Unreal Header Tool errors
- copied plugin/module load log lines
- the exact Blueprint node or class name that is missing
- the exact step where runtime behavior diverged

Do **not** summarize aggressively on the first retry. Raw error text is better than a confident paraphrase.

## Failure categories and next prompts

### 1) Build.cs or module dependency errors

Typical clues:

- missing module dependency
- unresolved include path
- linker error after adding a new engine subsystem or library

Prompt shape:

```text
Review this Unreal build failure using only the attached local corpus plus the error text.
Tell me the smallest likely fix first.
If the issue is probably Build.cs-related, explain which dependency or module boundary is most suspect and why.

Error:
[paste exact error]
```

## 2) Unreal Header Tool / reflection issues

Typical clues:

- UHT parse failures
- macro placement errors
- invalid `UCLASS`, `UFUNCTION`, or `UPROPERTY` usage
- generated code mismatch after type changes

Prompt shape:

```text
Review this Unreal Header Tool or reflection error against the attached plugin files.
Point to the exact header or macro pattern that is most likely wrong.
Do not guess beyond the evidence.

Error:
[paste exact UHT error]
```

## 3) Editor load or plugin enablement failures

Typical clues:

- plugin refuses to enable
- module fails during startup
- editor requests restart repeatedly
- log shows missing class/module warnings

Prompt shape:

```text
I enabled the plugin and got the following Unreal log lines.
Use the attached plugin corpus to produce a short root-cause checklist in probability order.
Separate module-load issues from class-registration issues.

Logs:
[paste exact lines]
```

## 4) Blueprint discovery problems

Typical clues:

- expected Blueprint library nodes do not appear
- subsystem/class is not visible where expected
- function names differ from what the answer implied

Prompt shape:

```text
Using only the attached plugin files, explain whether these Blueprint-facing symbols should be discoverable and what conditions would hide them.
Call out any uncertainty.

Observed issue:
[describe exact missing node/class]
```

## 5) Runtime behavior mismatches

Typical clues:

- event buffer count does not change
- flush behavior does not match comments
- logs appear in a different place than expected
- behavior only works in one lifecycle path

Prompt shape:

```text
Compare this observed runtime behavior against the attached subsystem and Blueprint library implementation.
List the most likely explanation in the code first, then the engine-validation steps to confirm it.

Observed behavior:
[describe exact behavior]
```

## Minimum evidence bundle before asking for help again

Try to capture at least this much:

- Unreal version
- validation path used: IDE build / UBT / editor compile
- exact error text or exact log lines
- file you changed, if any
- whether the problem happens before build, during load, or during runtime

## When to stop and re-ground

Stop asking for more edits and re-ground the workflow when:

- the answer starts inventing engine automation you did not run
- it recommends broad module splits without repo evidence
- it claims packaging/export behavior not shown in the code
- it keeps changing multiple files before the first compile problem is resolved

In those cases, attach the plugin files again and ask for the **smallest plausible fix only**.
