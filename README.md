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
- Workflow recipes, prompt templates, and the Blender scene cleanup demo kit: [`docs/workflows/README.md`](docs/workflows/README.md), [`docs/examples/blender-scene-cleanup-demo/README.md`](docs/examples/blender-scene-cleanup-demo/README.md)
- First-run/bootstrap notes: [`rust/BOOTSTRAP.md`](rust/BOOTSTRAP.md)
- Release checklist: [`rust/RELEASE.md`](rust/RELEASE.md)
- Changelog: [`rust/CHANGELOG.md`](rust/CHANGELOG.md)
- Artifact + privacy notes: [`rust/docs/ARTIFACTS.md`](rust/docs/ARTIFACTS.md), [`rust/docs/PRIVACY.md`](rust/docs/PRIVACY.md)

---

## Stable branch and CI posture

- **Stable branch:** `main`
- **Primary CI:** `.github/workflows/rust-ci.yml`
- **Pinned Rust toolchain:** [`rust/rust-toolchain.toml`](rust/rust-toolchain.toml) (`1.94.1` with `clippy` and `rustfmt`)
- **Expected verification flow:**
  1. `cd rust`
  2. `cargo build --workspace --locked`
  3. `cargo fmt --all --check`
  4. `cargo clippy --workspace --all-targets --locked`
  5. `cargo test --workspace --locked`

If you are reviewing the repository remotely, treat the Rust workspace as the source of truth for build/test health.

## Repository layout

```text
.
├── rust/                               # primary runnable harness + CI target
├── docs/                               # workflow recipes, prompts, examples
├── src/                                # secondary Python parity/porting workspace
├── tests/                              # Python-side verification for the porting workspace
├── assets/                             # images and supporting assets
├── FINAL_STATUS.md                     # honest maturity snapshot
├── NEXT_ACTIONS.md                     # prioritized follow-up work
└── README.md
```

## Python workspace status

The Python tree under `src/` remains in the repository as parity/porting work and reference material, but it is **not** the primary production-readiness surface right now.

Use it for historical comparison, parity exploration, and supplemental experimentation. Use `rust/` for the current harness, release posture, and CI expectations.

## Remote reviewer quickstart

If you just want to verify the current stable path on `main`:

```bash
git clone https://github.com/tinameriana009/Openclaw.git
cd Openclaw/rust
cargo build --workspace --locked
cargo test --workspace --locked
```

Then read:

- [`rust/README.md`](rust/README.md)
- [`rust/RELEASE.md`](rust/RELEASE.md)
- [`FINAL_STATUS.md`](FINAL_STATUS.md)
- [`NEXT_ACTIONS.md`](NEXT_ACTIONS.md)

## Development notes

This repository has been heavily AI-assisted during exploration, refactoring, validation, and documentation work. The important practical distinction is:

- the **Rust workspace** under `rust/` is the main active harness surface
- the **Python workspace** under `src/` is retained, but secondary
- the root docs should describe the runnable system honestly and point operators to the Rust path first

## Community / ownership

If you fork or continue this work, update the docs and repository metadata to match your own hosting and workflow.

Current repository home:
- <https://github.com/tinameriana009/Openclaw>

## Ownership / affiliation disclaimer

- This repository does **not** claim ownership of the original Claude Code source material.
- This repository is **not affiliated with, endorsed by, or maintained by Anthropic**.
