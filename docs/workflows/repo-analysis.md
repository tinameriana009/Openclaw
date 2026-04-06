# Workflow recipe: general repository analysis

Use this when you want `claw` to act like a grounded repo analyst instead of a code generator.

This is the strongest current fit for the harness because it combines:

- local corpus attachment
- recursive answer support
- execution profiles
- trace export
- session resume

## High-value use cases

- onboarding into an unfamiliar repository
- summarizing architecture or subsystem boundaries
- tracing where a feature is implemented
- reviewing likely risks before refactoring
- producing a handoff brief for another engineer

## Fastest useful start

```bash
cd rust
./target/debug/claw --profile balanced \
  --corpus .. \
  prompt "Summarize this repository's main subsystems, likely entrypoints, and risky areas for modification"
```

If the repository is large or the question is cross-cutting, bump to `deep` or `research`.

## Recommended flow

### 1) Start broad

Use the template in [`../prompts/repo-analysis-task.md`](../prompts/repo-analysis-task.md).

Example:

```text
Analyze the attached repository.
Ground your answer in the local corpus and be explicit about uncertainty.

Output:
1. Top-level system summary
2. Likely entrypoints
3. Main subsystems or crates/modules
4. Important configs, data directories, or generated artifacts
5. High-risk areas for change
6. Questions worth answering next
```

### 2) Narrow to a feature path

Examples:

- `Trace the auth flow from CLI entrypoint to provider call.`
- `Show where session persistence is implemented.`
- `List files involved in corpus search and answer flow.`
- `Map the trace export path end to end.`

### 3) Ask for actionable outputs

Good asks include:

- a file shortlist to read next
- a refactor plan with risk notes
- a code review checklist
- a bug reproduction hypothesis
- a migration brief for a new contributor

### 4) Keep a session alive for deeper work

Large investigations go better if you resume instead of restarting:

```bash
./target/debug/claw --resume latest
./target/debug/claw --resume latest /status /diff
```

Inside the REPL:

```text
/session list
/resume latest
```

### 5) Use traces when you want evidence of the reasoning path

```bash
ls .claw/trace
./target/debug/claw --resume latest /trace summary .claw/trace/<trace-file>
```

This is especially helpful when the answer combines many files or corpus slices.

## Suggested query ladder

A reliable progression:

1. `Summarize the repo.`
2. `List the key files for subsystem X.`
3. `Trace feature Y across those files.`
4. `Identify risks in changing feature Y.`
5. `Propose a minimal implementation plan.`

## Good follow-up prompts

- `Turn this architecture summary into a new-contributor onboarding note.`
- `List likely dead code or stale integration points.`
- `Compare the documented flow with the implemented flow.`
- `Find where this config value is read and propagated.`
- `Produce a test impact map for changing this module.`

## Example deliverables to ask for

- architecture digest
- dependency map by subsystem
- feature trace report
- refactor plan
- risk register
- onboarding memo

## Current workflow limits

- no graphical architecture explorer yet
- no first-class saved workflow runner beyond normal sessions and prompts
- very large repos still depend on corpus selection quality and prompt discipline
- best trace UX is still the saved ledger on disk, not a fully polished in-app viewer
