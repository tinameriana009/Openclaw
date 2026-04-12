# Trace review checklist: repo analysis demo

Use this when the answer feels plausible but you want to inspect how the grounded path was built.

## What to look for in the trace

- whether the model actually touched the files it cites
- whether it relied on summary files instead of the implementation files
- whether it skipped over contradictory or ambiguous evidence
- whether a broad architecture claim came from one thin slice
- whether a replay/resume pass introduced genuinely new evidence or just restated the old answer

## Useful operator questions

- Did the trace include `src/main.py` before making CLI claims?
- Did it inspect `src/runtime.py` or `src/execution_registry.py` before describing session/bootstrap behavior?
- Did it treat `tests/test_porting_workspace.py` as evidence for public behavior?
- Did it cite `src/reference_data/` when discussing mirrored commands/tools or parity metadata?
- Did the later pass materially differ from the prior reviewed run, or only sound more confident?

## When to re-prompt

Re-prompt if the trace shows:

- too much reliance on one file for cross-cutting conclusions
- confident architecture claims with weak file evidence
- no test evidence for output-contract claims
- no uncertainty callouts despite obvious ambiguity
- a replay/resume pass that repeated the same claim without new file evidence
- a handoff bundle that points at a prior reviewed run but does not explain what changed this time
