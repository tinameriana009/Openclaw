# Repo analysis demo kit

This demo kit gives the repository-analysis workflow a more realistic, repeatable operator loop.

It does **not** pretend the harness can verify architecture claims automatically. Instead, it gives you:

- a concrete brief to run against a real local corpus
- an expected-findings sheet to compare against
- a manual validation checklist for reviewing the answer
- an operator session template for capturing evidence and handoff notes
- a next-prompt template for turning review findings into the next grounded ask
- a trace-review checklist for auditing how the answer was produced

## Corpus target

Use the Python parity workspace in this repository as the local corpus root:

- `src/`
- optionally `tests/`
- optionally `README.md` and `PARITY.md`

That target is intentionally modest: it is large enough to require real repo analysis, but small enough for a new operator to validate manually.

## Suggested command sequence

Fastest realistic path from `rust/` after building `claw`:

```bash
./scripts/run-repo-analysis-demo.sh
```

That helper:

- runs the documented onboarding brief against `../src` and `../tests`
- resumes the same session for the file-path follow-up
- saves both responses under `.demo-artifacts/repo-analysis-demo/<timestamp>/`
- stages `operator-session-template.md`, `next-prompt-template.md`, and a findings template alongside the run outputs
- emits `bundle-summary.json`, `operator-handoff.json`, `bundle-checksums.txt`, and a static `operator-dashboard.html` so the next operator has a durable review/resume bundle
- prints the next validation/trace-review steps instead of pretending the run is self-certifying

Optional overrides:

```bash
PROFILE=deep ./scripts/run-repo-analysis-demo.sh
ARTIFACT_ROOT=/tmp/repo-demo ./scripts/run-repo-analysis-demo.sh
```

If you prefer the raw commands, they are:

```bash
./target/debug/claw --profile balanced \
  --corpus ../src \
  --corpus ../tests \
  prompt "Analyze the attached repository for a new engineer. Summarize the main entrypoints, key subsystems, important generated/reference data, and the riskiest areas to modify. End with a suggested reading order."
```

Then tighten the ask:

```bash
./target/debug/claw --resume latest \
  prompt "Now produce a file-level handoff note for someone changing the query, runtime, or execution-registry paths. Distinguish facts from inferences."
```

If you want a richer reasoning trail, repeat with `PROFILE=deep` or `PROFILE=research` in the helper, or re-run the raw flow with `--profile deep` / `--profile research`.

## Validate the output

1. Start with [`brief.md`](brief.md).
2. Compare the answer against [`expected-findings.md`](expected-findings.md).
3. Use [`manual-validation-checklist.md`](manual-validation-checklist.md) while reading the referenced files.
4. Capture exact evidence, weak spots, and handoff notes in [`operator-session-template.md`](operator-session-template.md).
5. Open `operator-dashboard.html` or inspect `bundle-summary.json` / `operator-handoff.json` if you are handing the run to another operator or picking it back up later.
6. If the model made a surprising jump, inspect the trace using [`trace-review-checklist.md`](trace-review-checklist.md).
7. Turn the review into the next grounded question with [`next-prompt-template.md`](next-prompt-template.md).

## Fast local coherence check

Before handing this demo kit to someone else, run:

```bash
python3 tests/validate_repo_analysis_demo.py
```

That check only verifies the demo assets and doc wiring. It does **not** score model quality for you.

## Review / resume continuity

The staged run bundle is meant to survive operator handoff honestly:

- `operator-dashboard.html` is a static on-disk dashboard for the run, not a live web UI.
- `bundle-summary.json` lists the run profile, bundle files, and exact continuity commands.
- `operator-handoff.json` captures the minimum payload another operator needs to continue review.
- `bundle-checksums.txt` lets the next operator confirm the staged bundle was not silently changed.

For a continued pass, prefer resuming the same session and grounding the next prompt in what you already verified manually. If you need the bounded trace lifecycle commands, use `/trace replay ...` and `/trace resume ...`; those preserve CLI continuity, but they still do **not** imply browser automation.
