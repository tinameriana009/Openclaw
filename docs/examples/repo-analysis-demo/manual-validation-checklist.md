# Manual validation checklist: repo analysis demo

Use this after the model answers.

## Basic grounding

- [ ] Did the answer name `src/main.py` as an entrypoint or central CLI surface?
- [ ] Did it cite specific files instead of only package names?
- [ ] Did it distinguish facts from guesses where the architecture is not explicit?

## Important surfaces

- [ ] Did it mention `src/query_engine.py` or `QueryEnginePort` for analysis/summary behavior?
- [ ] Did it mention `src/runtime.py` for bootstrap/runtime session behavior?
- [ ] Did it mention `src/execution_registry.py` for mirrored command/tool execution?
- [ ] Did it mention `src/reference_data/` as a generated/reference snapshot area?

## Risk analysis quality

- [ ] Did it explain *why* a file is risky to change, not just list it?
- [ ] Did it acknowledge that CLI output contracts are test-sensitive?
- [ ] Did it mention the tests as an evidence source or behavior oracle?

## Suggested file spot-checks

Open these files while reviewing the answer:

- `src/main.py`
- `src/query_engine.py`
- `src/runtime.py`
- `src/execution_registry.py`
- `tests/test_porting_workspace.py`

## Honest workflow limits

- [ ] Did the answer avoid claiming runtime behavior that is not visible in the local corpus?
- [ ] Did it avoid pretending the harness executed or verified the repo automatically?
