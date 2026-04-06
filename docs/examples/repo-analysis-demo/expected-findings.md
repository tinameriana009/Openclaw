# Expected findings for the repo analysis demo

Use this as a grounding check, not a grading rubric. The model may phrase things differently.

## Things a good answer should usually notice

### Entry points

- `src/main.py` is the practical CLI entrypoint for the Python parity workspace.
- The tests exercise many workflows by running `python -m src.main ...`.

### Query / summary surface

- `src/query_engine.py` exposes `QueryEnginePort` and summary rendering for the workspace.
- `src/port_manifest.py` builds a manifest over the Python files.
- `src/parity_audit.py` computes coverage-style parity audit information.

### Runtime / bootstrap surface

- `src/runtime.py` defines the bootstrap session machinery used by tests and CLI helpers.
- `src/execution_registry.py` builds a mirrored registry for commands and tools.
- `src/session_store.py` and `src/transcript.py` are part of persistence / transcript handling.

### Command and tool mirroring

- `src/commands.py` and `src/tools.py` expose large mirrored command/tool registries.
- Reference snapshots live under `src/reference_data/`.

### Risky areas to change

- CLI dispatch and output contracts in `src/main.py`, because many tests assert against them.
- Registry-building behavior in `src/execution_registry.py`.
- Session bootstrap/runtime behavior in `src/runtime.py`.
- Snapshot assumptions in `src/reference_data/` and the code that reads them.

### Good reading order

A sensible reading order is often:

1. `README.md`
2. `src/main.py`
3. `src/query_engine.py`
4. `src/runtime.py`
5. `src/execution_registry.py`
6. `src/commands.py` and `src/tools.py`
7. `tests/test_porting_workspace.py`

## Signs the answer is too hand-wavy

Be skeptical if the answer:

- never names `src/main.py`
- talks about architecture without pointing to files
- ignores `src/reference_data/`
- misses that the test suite is a major source of truth for expected behavior
