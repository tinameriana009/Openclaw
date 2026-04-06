# Openclaw

<p align="center">
  <img src="assets/clawd-hero.jpeg" alt="Claw" width="300" />
</p>

<p align="center">
  <strong>Rust-first agent harness work with local corpus RAG, recursive runtime flows, trace export, and CLI/REPL tooling.</strong>
</p>

> [!IMPORTANT]
> The practical operator/developer docs for the current Rust harness live in [`rust/README.md`](rust/README.md). If you want to build, run, test, or evaluate the current system — especially the newer `--profile`, corpus, recursive, and trace flows — start there.

> [!NOTE]
> This repository is maintained at **tinameriana009/Openclaw**. The current local/alpha work is centered on the Rust harness under `rust/`.

## What is better in this repository?

Compared with the earlier parity baseline, this repository now includes substantial additional work:

- **Local corpus RAG**
  - attach, persist, inspect, search, and slice local corpora
  - CLI and REPL corpus flows such as `--corpus` and `/corpus ...`
- **Recursive runtime improvements**
  - bounded recursive/iterative execution paths
  - provider-backed child subquery flow for corpus answering
  - stronger traceability for recursive runs
- **Trace + telemetry**
  - structured trace ledger export
  - counters and telemetry-safe summaries
  - better debugging of grounded multi-step flows
- **Execution profiles**
  - `fast`, `balanced`, `deep`, and `research`
  - profile-aware RAG / RLM / web behavior
- **Web-aware hybrid scaffolding**
  - web policy, evidence normalization, escalation heuristics, and provenance support
- **Operator-focused docs and bootstrap notes**
  - current Rust operator guide in [`rust/README.md`](rust/README.md)
  - first-run notes in [`rust/BOOTSTRAP.md`](rust/BOOTSTRAP.md)
- **Fork-specific integration work**
  - cross-milestone integration beyond the baseline parity snapshot
  - practical hardening around CLI help, corpus flows, trace flows, and local usability

In short: this repo is no longer just a parity snapshot. It is evolving into a more usable **custom-task agent harness** for grounded local work, recursive analysis, and future hybrid local+web workflows.

---

## Current Focus

This repository is focused on a practical agent harness with:

- interactive CLI / REPL flows
- session persistence and resume
- permission and sandbox controls
- local corpus attachment, search, inspect, and slice flows
- recursive runtime scaffolding for grounded multi-step answers
- trace ledger export and telemetry
- execution profiles (`fast`, `balanced`, `deep`, `research`)

The most current implementation surface is the **Rust workspace** under [`rust/`](rust/).

## Repository Home

- GitHub: <https://github.com/tinameriana009/Openclaw>
- Main operator docs: [`rust/README.md`](rust/README.md)
- Workflow recipes and prompt templates: [`docs/workflows/README.md`](docs/workflows/README.md)
- First-run/bootstrap notes: [`rust/BOOTSTRAP.md`](rust/BOOTSTRAP.md)
- Release checklist: [`rust/RELEASE.md`](rust/RELEASE.md)
- Changelog: [`rust/CHANGELOG.md`](rust/CHANGELOG.md)
- Artifact + privacy notes: [`rust/docs/ARTIFACTS.md`](rust/docs/ARTIFACTS.md), [`rust/docs/PRIVACY.md`](rust/docs/PRIVACY.md)

---

## Porting Status

The main source tree is now Python-first.

- `src/` contains the active Python porting workspace
- `tests/` verifies the current Python workspace
- the exposed snapshot is no longer part of the tracked repository state

The current Python workspace is not yet a complete one-to-one replacement for the original system, but the primary implementation surface is now Python.

## Why this rewrite exists

I originally studied the exposed codebase to understand its harness, tool wiring, and agent workflow. After spending more time with the legal and ethical questions—and after reading the essay linked below—I did not want the exposed snapshot itself to remain the main tracked source tree.

This repository now focuses on Python porting work instead.

## Repository Layout

```text
.
├── src/                                # Python porting workspace
│   ├── __init__.py
│   ├── commands.py
│   ├── main.py
│   ├── models.py
│   ├── port_manifest.py
│   ├── query_engine.py
│   ├── task.py
│   └── tools.py
├── tests/                              # Python verification
├── assets/omx/                         # OmX workflow screenshots
├── 2026-03-09-is-legal-the-same-as-legitimate-ai-reimplementation-and-the-erosion-of-copyleft.md
└── README.md
```

## Python Workspace Overview

The new Python `src/` tree currently provides:

- **`port_manifest.py`** — summarizes the current Python workspace structure
- **`models.py`** — dataclasses for subsystems, modules, and backlog state
- **`commands.py`** — Python-side command port metadata
- **`tools.py`** — Python-side tool port metadata
- **`query_engine.py`** — renders a Python porting summary from the active workspace
- **`main.py`** — a CLI entrypoint for manifest and summary output

## Quickstart

Render the Python porting summary:

```bash
python3 -m src.main summary
```

Print the current Python workspace manifest:

```bash
python3 -m src.main manifest
```

List the current Python modules:

```bash
python3 -m src.main subsystems --limit 16
```

Run verification:

```bash
python3 -m unittest discover -s tests -v
```

Run the parity audit against the local ignored archive (when present):

```bash
python3 -m src.main parity-audit
```

Inspect mirrored command/tool inventories:

```bash
python3 -m src.main commands --limit 10
python3 -m src.main tools --limit 10
```

## Current Parity Checkpoint

The port now mirrors the archived root-entry file surface, top-level subsystem names, and command/tool inventories much more closely than before. However, it is **not yet** a full runtime-equivalent replacement for the original TypeScript system; the Python tree still contains fewer executable runtime slices than the archived source.


## Development Notes

This repository has been heavily AI-assisted during exploration, refactoring, validation, and documentation work. The important thing for operators is the current state of the tree:

- the **Rust workspace** under `rust/` is the main active harness surface
- the **Python porting workspace** under `src/` remains part of the repository history and parity exploration
- current docs are being updated to reflect the real runnable system rather than the older backstory-heavy presentation

## Community / Ownership

If you fork or continue this work, update the docs and repository metadata to match your own hosting and workflow.

Current repository home:
- <https://github.com/tinameriana009/Openclaw>

## Ownership / Affiliation Disclaimer

## Ownership / Affiliation Disclaimer

- This repository does **not** claim ownership of the original Claude Code source material.
- This repository is **not affiliated with, endorsed by, or maintained by Anthropic**.
