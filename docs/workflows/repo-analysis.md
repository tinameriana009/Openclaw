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

For the lowest-friction realistic demo path, use the helper wrapper first:

```bash
cd rust
cargo build --workspace --locked
./scripts/run-repo-analysis-demo.sh
```

That runs the documented two-step prompt flow, resumes the same session for the follow-up, and captures both responses under `.demo-artifacts/repo-analysis-demo/<timestamp>/` for later review.
It also stages `bundle-summary.json`, `operator-handoff.json`, `review-status.json`, `review-log.md`, `bundle-checksums.txt`, and a static `operator-dashboard.html`, then refreshes `.demo-artifacts/repo-analysis-demo/index.{json,html}` so another operator can review/resume the same run and compare it against earlier passes without guessing what happened.

If you want the raw one-liner instead, use:

```bash
cd rust
./target/debug/claw --profile balanced \
  --corpus .. \
  prompt "Summarize this repository's main subsystems, likely entrypoints, and risky areas for modification"
```

If the repository is large or the question is cross-cutting, bump to `deep` or `research`.

If you want a more realistic operator path instead of a generic one-liner, use the repo analysis demo kit:

- helper runner: [`../../rust/scripts/run-repo-analysis-demo.sh`](../../rust/scripts/run-repo-analysis-demo.sh)

- [`../examples/repo-analysis-demo/README.md`](../examples/repo-analysis-demo/README.md)
- [`../examples/repo-analysis-demo/brief.md`](../examples/repo-analysis-demo/brief.md)
- [`../examples/repo-analysis-demo/expected-findings.md`](../examples/repo-analysis-demo/expected-findings.md)
- [`../examples/repo-analysis-demo/manual-validation-checklist.md`](../examples/repo-analysis-demo/manual-validation-checklist.md)
- [`../examples/repo-analysis-demo/operator-session-template.md`](../examples/repo-analysis-demo/operator-session-template.md)
- [`../examples/repo-analysis-demo/next-prompt-template.md`](../examples/repo-analysis-demo/next-prompt-template.md)
- [`../examples/repo-analysis-demo/trace-review-checklist.md`](../examples/repo-analysis-demo/trace-review-checklist.md)
- staged run dashboard: `.demo-artifacts/repo-analysis-demo/<timestamp>/operator-dashboard.html`
- staged handoff metadata: `.demo-artifacts/repo-analysis-demo/<timestamp>/{bundle-summary.json,operator-handoff.json,review-status.json,review-log.md,bundle-checksums.txt}`
- cross-run static index: `.demo-artifacts/repo-analysis-demo/{index.json,index.html}`

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

For a more repeatable review loop, use the repo-analysis trace checklist in [`../examples/repo-analysis-demo/trace-review-checklist.md`](../examples/repo-analysis-demo/trace-review-checklist.md).

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

## Manual validation loop

A good repo-analysis workflow should end with verification, not just a plausible summary.

After the model answers:

1. compare it against the known-good anchors in [`../examples/repo-analysis-demo/expected-findings.md`](../examples/repo-analysis-demo/expected-findings.md)
2. spot-check the referenced files using [`../examples/repo-analysis-demo/manual-validation-checklist.md`](../examples/repo-analysis-demo/manual-validation-checklist.md)
3. capture exact evidence, missed files, and weak claims in [`../examples/repo-analysis-demo/operator-session-template.md`](../examples/repo-analysis-demo/operator-session-template.md)
4. inspect the trace if the model made a broad claim from thin evidence
5. re-prompt with narrower file targets when needed using [`../examples/repo-analysis-demo/next-prompt-template.md`](../examples/repo-analysis-demo/next-prompt-template.md)
6. if another operator is taking over, hand them the staged `operator-dashboard.html`, `operator-handoff.json`, `review-status.json`, and `review-log.md` instead of a loose summary message
7. if you need to compare runs or see which bundles were actually reviewed, open `.demo-artifacts/repo-analysis-demo/index.html`

You can also run a lightweight coherence check for the demo assets themselves:

```bash
python3 tests/validate_repo_analysis_demo.py
```

That validates the docs/examples wiring, not the model's quality.

## Current workflow limits

- no graphical architecture explorer yet
- no first-class saved workflow runner beyond normal sessions and prompts
- very large repos still depend on corpus selection quality and prompt discipline
- best trace/review UX is still the saved ledger on disk plus the staged static run dashboard and cross-run index, not a fully polished live in-app viewer
- architecture validation is still operator-driven; the harness does not automatically certify that its summary is correct
