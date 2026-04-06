# Repo analysis demo kit

This demo kit gives the repository-analysis workflow a more realistic, repeatable operator loop.

It does **not** pretend the harness can verify architecture claims automatically. Instead, it gives you:

- a concrete brief to run against a real local corpus
- an expected-findings sheet to compare against
- a manual validation checklist for reviewing the answer
- a trace-review checklist for auditing how the answer was produced

## Corpus target

Use the Python parity workspace in this repository as the local corpus root:

- `src/`
- optionally `tests/`
- optionally `README.md` and `PARITY.md`

That target is intentionally modest: it is large enough to require real repo analysis, but small enough for a new operator to validate manually.

## Suggested command sequence

From `rust/` after building `claw`:

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

If you want a richer reasoning trail, repeat with `--profile deep` or `--profile research`.

## Validate the output

1. Start with [`brief.md`](brief.md).
2. Compare the answer against [`expected-findings.md`](expected-findings.md).
3. Use [`manual-validation-checklist.md`](manual-validation-checklist.md) while reading the referenced files.
4. If the model made a surprising jump, inspect the trace using [`trace-review-checklist.md`](trace-review-checklist.md).

## Fast local coherence check

Before handing this demo kit to someone else, run:

```bash
python3 tests/validate_repo_analysis_demo.py
```

That check only verifies the demo assets and doc wiring. It does **not** score model quality for you.
