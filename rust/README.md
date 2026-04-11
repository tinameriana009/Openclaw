# Claw Code Rust Workspace

This `rust/` tree is the current Rust harness implementation for `claw`.

If you want to install it, authenticate, attach a local corpus, run a task, and inspect traces without spelunking through source, start here.

## Current honest status

- **Maturity:** strong alpha / early pre-production candidate
- **Best current use:** grounded local technical tasks over repos, docs, and attached corpora
- **Verified state:** `cargo build --workspace --locked` and `cargo test --workspace --locked` currently pass from this `rust/` workspace root with the pinned toolchain
- **Not yet:** a fully production-ready universal agent platform

Strongest current areas:
- local corpus RAG
- recursive runtime hardening
- trace export and telemetry
- operator/release/trust docs
- workflow demos for Blender, repo analysis, and Unreal

Best current showcase path:
- the repo-analysis flow, especially via `./scripts/run-repo-analysis-demo.sh`, because it exercises grounded local analysis, session resume, trace review, and a static operator handoff/dashboard bundle without depending on Blender or Unreal manual UI loops

Most realistic operator-prep helpers for the more manual workflows:
- `./scripts/prepare-blender-demo.sh` validates the Blender demo kit, rebuilds the zip artifact, and stages a manual-review bundle under `.demo-artifacts/blender-demo/` with findings/prompt templates plus `bundle-summary.json`, `operator-handoff.json`, and `bundle-checksums.txt` for the next handoff
- `./scripts/prepare-unreal-demo.sh` validates the Unreal demo kit and stages a plugin/checklist bundle under `.demo-artifacts/unreal-demo/` with findings/prompt templates plus `bundle-summary.json`, `operator-handoff.json`, and `bundle-checksums.txt` for the next handoff

## What ships today

Operator trust/release docs added for this harness:

- [`CHANGELOG.md`](CHANGELOG.md) — release notes scaffold and current baseline notes
- [`RELEASE.md`](RELEASE.md) — repeatable release/readiness checklist
- [`docs/RELEASE_CANDIDATE.md`](docs/RELEASE_CANDIDATE.md) — the bounded RC gate and release-note minimum for this repo
- [`docs/ARTIFACTS.md`](docs/ARTIFACTS.md) — on-disk artifact contract and compatibility notes
- [`docs/PRIVACY.md`](docs/PRIVACY.md) — privacy and handling guidance for `.claw/` artifacts


The current CLI help and local smoke tests confirm these operator-facing surfaces:

- Interactive REPL (`claw`)
- Non-interactive prompt mode (`claw prompt ...` and `claw "..."`)
- Status / sandbox inspection (`claw status`, `claw sandbox`)
- OAuth login / logout (`claw login`, `claw logout`)
- Session autosave and resume (`--resume`, `/session`, `/resume`)
- Permission controls and allowed-tool filtering
- Local corpus attachment plus `/corpus` search / inspect / slice flows
- Execution profiles: `fast`, `balanced`, `deep`, `research`
- Recursive trace inspection via `/trace ...` and saved trace ledgers under `.claw/trace/`

## Fastest honest first run

From a clean host:

```bash
cd rust
cargo build --workspace --locked
./target/debug/claw --help
./target/debug/claw status
```

If that works, you have a runnable local install.

## Install / build

### Toolchain requirement

This workspace uses a modern Cargo lockfile and is pinned via [`rust-toolchain.toml`](rust-toolchain.toml).
On older Ubuntu hosts, `/usr/bin/cargo` may be too old and fail with a lockfile-v4 error.

If that happens:

```bash
curl https://sh.rustup.rs -sSf | sh
. "$HOME/.cargo/env"
rustup toolchain install 1.94.1 --profile minimal --component clippy --component rustfmt
cd rust
rustup override set 1.94.1
cargo --version
rustc --version
```

If your shell still resolves the distro Cargo first, use `~/.cargo/bin/cargo` explicitly.

### Build + verify

Important:
- run verification from the `rust/` directory, not the repository root
- on older hosts, prefer `~/.cargo/bin/cargo` or `. "$HOME/.cargo/env"` first if the system Cargo is too old for this workspace

Preferred:

```bash
cd rust
./scripts/release-verify.sh
```

For release-candidate discipline instead of a normal smoke pass:

```bash
cd rust
RELEASE_CANDIDATE=1 ./scripts/release-verify.sh
```

That helper checks the active Rust toolchain first, then runs the locked workspace verification sequence. In RC mode it also runs the explicit release-candidate documentation gate, generates `.claw/release-artifacts/release-manifest.json` plus `.claw/release-artifacts/release-attestation.json`, and expects release notes/trust docs to stay aligned with the current `artifactKind` / `schemaVersion` / `compatVersion` contract. If you provide `PROVENANCE_SIGNING_KEY=/path/to/private-key.pem`, the same flow will also emit `.claw/release-artifacts/release-provenance.json`, `.claw/release-artifacts/release-provenance.sig`, and `.claw/release-artifacts/release-trust-policy.json`, then verify that signed bundle locally. If you additionally provide `PROVENANCE_SIGNING_CERT=/path/to/leaf-cert.pem` plus `PROVENANCE_TRUST_ROOT=/path/to/root-ca.pem` (and optionally `PROVENANCE_SIGNING_CHAIN=/path/to/intermediate-chain.pem`), that bundle will also pin and validate a supplied X.509 chain back to the provided root. If you additionally set `EXTERNAL_PROVENANCE_WITNESS=1`, the release verification flow will also emit `.claw/release-artifacts/release-external-witness.json` and validate it as a bounded external repository/publication anchor layer over the signed bundle. This is still bounded release provenance, not a claim of Sigstore/SLSA-grade hosted supply-chain security.

Manual equivalent:

```bash
cd rust
cargo build --workspace --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked
cargo test --workspace --locked
python3 ../tests/validate_operator_readiness.py
python3 ../tests/validate_blender_demo.py
python3 ../tests/validate_unreal_demo.py
python3 ../tests/validate_repo_analysis_demo.py
```

After a successful build, the binary is:

```bash
./target/debug/claw
```

## Authentication

Two supported paths:

### 1) API key

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

### 2) OAuth login

```bash
cd rust
./target/debug/claw login
```

Current observed behavior:

- starts a local callback server on `http://localhost:4545/callback`
- prints an OAuth authorize URL
- tries to open a browser automatically
- if no browser opener is available, you can open the printed URL manually

Use logout to clear saved auth:

```bash
./target/debug/claw logout
```

## Operator quickstart

### 1) Confirm local status

```bash
cd rust
./target/debug/claw status
./target/debug/claw sandbox
```

`status` is the best single-command smoke test for:

- model selection
- permission mode
- active profile
- workspace detection
- git visibility
- config / memory discovery

### 2) Run a one-shot task

```bash
./target/debug/claw prompt "summarize this repo"
./target/debug/claw "summarize crates/runtime/src/config.rs"
./target/debug/claw --output-format json prompt "summarize src/main.rs"
```

### 3) Start the REPL

```bash
./target/debug/claw
```

Inside the REPL, start with:

```text
/help
/status
```

## Common flags

These are the main flags exposed by `claw --help`:

```bash
./target/debug/claw --model claude-opus "summarize this repo"
./target/debug/claw --permission-mode read-only status
./target/debug/claw --dangerously-skip-permissions status
./target/debug/claw --profile deep prompt "trace the recursive answer path"
./target/debug/claw --allowedTools read,glob "summarize Cargo.toml"
./target/debug/claw --output-format json prompt "show a machine-readable summary"
```

### Profiles

| Profile | Intended use | Notes |
|---|---|---|
| `fast` | Cheap/local quick answers | Lowest budget |
| `balanced` | Default daily mode | Balanced enables recursive trace capture by default |
| `deep` | Heavier local investigation | More depth / budget |
| `research` | Most expensive local investigation | Largest recursive budget |

## Corpus workflow

The current operator story is:

1. attach one or more local corpus roots with `--corpus` or `/corpus attach`
2. inspect/search them from the REPL
3. run a grounded query
4. inspect saved trace artifacts

### Attach corpus roots up front

```bash
./target/debug/claw --corpus ./docs --corpus ./crates/runtime
./target/debug/claw --corpus ./docs --profile research prompt "What changed in auth flow?"
```

### Discover corpus commands in the REPL

```text
/corpus
/corpus attach ./docs
/corpus search auth callback
/corpus inspect <corpus-id>
/corpus slice <chunk-id>
```

The top-level help currently advertises this grounded path as the main corpus answer flow:

```text
/corpus answer <query>
```

That is the discoverability path operators should try first when they want a grounded recursive answer over attached local material.

## Sessions and resume

Sessions auto-save under:

```text
.claw/sessions/<session-id>.jsonl
```

Useful commands:

```bash
./target/debug/claw --resume latest
./target/debug/claw --resume latest /status
./target/debug/claw --resume latest /diff /export notes.txt
```

Inside the REPL:

```text
/session list
/resume latest
```

## Traces and artifacts

For trace-enabled runs, inspect artifacts under:

- `.claw/trace/` — recursive trace ledgers
- `.claw/telemetry/recursive-runtime.jsonl` — recursive runtime telemetry
- `.claw/sessions/` — autosaved sessions

### Recommended trace flow

1. run with a trace-capable profile such as `balanced`, `deep`, or `research`
2. attach local corpus roots if you want grounded answers
3. execute the task
4. inspect stdout for the trace summary
5. open the saved ledger in `.claw/trace/`

The CLI help also exposes:

```text
/trace summary <trace-file>
/trace export <trace-file> [destination]
/trace approve <trace-file>
/trace replay <trace-file|approval-packet>
/trace resume <trace-file|approval-packet>
```

For the current bounded web-review loop:
- `/trace approve ...` records an approval packet plus `.review.{json,md}` artifacts
- `/trace replay ...` reruns an already approved packet and refreshes those review artifacts
- `/trace resume ...` is the honest approve-and-rerun shortcut; it still does **not** provide browser automation
- `.claw/web-approvals/index.{json,md,html}` acts as a lightweight on-disk operator dashboard for the saved review state, including explicit follow-up `/trace review|replay|resume` commands for each entry

## High-value slash commands

Start here:

```text
/help
/status
/sandbox
/diff
/commit
/agents
/skills
/corpus
/trace
```

Other useful operator commands from the current help surface:

```text
/model [model]
/permissions [read-only|workspace-write|danger-full-access]
/config [env|hooks|model|plugins]
/memory
/session [list|switch <session-id>|fork [branch-name]]
/export [file]
/mcp [list|show <server>|help]
```

## Config + workspace notes

- The CLI reports config discovery in `claw status`
- `status` also reports workspace root, branch, dirty state, and memory-file loading
- the current default permission mode is `workspace-write` unless config / env overrides it
- use `--dangerously-skip-permissions` only when you intentionally want unrestricted execution for that run

## Compatibility and migration notes

Current baseline:

- the workspace version is `0.1.0`
- new trace ledgers and corpus manifests now carry `artifactKind`, `schemaVersion`, and `compatVersion`
- the release artifact manifest now also carries a small local provenance envelope (`schemaVersion=2`, `compatVersion=0.2`) with remotes, host/toolchain snapshot, declared verification commands, and hashed release materials
- a paired `release-attestation.json` sidecar (`artifactKind=claw.release-attestation`, `schemaVersion=1`, `compatVersion=0.1`) binds the manifest hash and built binary hash into a more formal statement shape
- trace and corpus artifact formats are documented, but still pre-1.0 contracts
- there is not yet a dedicated migration layer for session / trace / corpus artifacts, and even the improved signed release path remains operator-supplied trust material rather than public transparency-backed provenance
- safest automation strategy today is pinning to a git tag or commit and parsing `.claw/` artifacts defensively

If you build tooling around `.claw/trace/`, `.claw/corpus/`, or `.claw/telemetry/`, read [`docs/ARTIFACTS.md`](docs/ARTIFACTS.md) first.
If you need to share traces or manifests outside your machine, read [`docs/PRIVACY.md`](docs/PRIVACY.md) first.

## Known operator gaps

These are the important remaining rough edges from an operator point of view:

- The install story is still source-first; there is no polished packaged release flow documented here yet.
- The quickest reliable trace workflow is still “inspect `.claw/trace/` on disk”; the CLI trace UX is improving but the saved artifact path remains the safest one to depend on.
- Corpus discoverability is much better than before, but the most advanced grounded-answer path should still be treated as an active harness surface rather than a finished product.
- Web-aware behavior is more honest and traceable than before, including explicit approval-required child states, a dedicated final-answer `Web execution` section with per-status counts/subquery details, richer provenance summaries, and static on-disk handoff/dashboard artifacts for the repo-analysis showcase, but it is still not a fully mature end-to-end operator web workflow.
- Child execution is cleaner and more shared than before, but not yet fully runtime/provider-owned in every seam.
- Recursive traces now capture novelty/progress signals per child step, retrieval requests carry lightweight planner metadata (strategy / rationale / anchor terms), and the final recursive answer surfaces a compact `Recursive planning` summary so operators can see what the heuristic planner was doing without opening the raw trace first.
- Follow-up retrieval planning is now slightly more disciplined: later iterations prefer structured `Findings`, `Validation loop`, and `Remaining gaps` terms across recent child responses, explicitly mark when the loop appears to be probing open gaps versus stalling on repeated ones, and preserve that planner-progress metadata in the trace/final answer. This is still heuristic guidance rather than a full planner or formal verifier.
- OAuth currently depends on a localhost callback and manual URL opening when no browser opener is available.

## Workspace layout

```text
rust/
├── Cargo.toml
├── Cargo.lock
├── rust-toolchain.toml
└── crates/
    ├── api/
    ├── commands/
    ├── compat-harness/
    ├── plugins/
    ├── runtime/
    ├── rusty-claude-cli/
    ├── telemetry/
    └── tools/
```

## Domain workflow recipes

If you want concrete operator playbooks instead of raw command reference, see:

- [`../docs/workflows/README.md`](../docs/workflows/README.md)
- [`../docs/workflows/blender-addon.md`](../docs/workflows/blender-addon.md)
- [`../docs/workflows/unreal-plugin.md`](../docs/workflows/unreal-plugin.md)
- [`../docs/workflows/repo-analysis.md`](../docs/workflows/repo-analysis.md)

Those recipes stay within the currently supported harness surface: profiles, local corpus attachment, grounded `/corpus answer ...` flows, sessions, and trace inspection.

If you need honest prep helpers for the more operator-heavy demos, run one of these first:

```bash
cd rust
./scripts/prepare-blender-demo.sh
./scripts/prepare-unreal-demo.sh
./scripts/prepare-domain-demo-bundles.sh
```

The combined domain helper stages both the Blender and Unreal bundles in one pass so operator handoff is more repeatable across app-heavy demos. All three helpers remain staged-handoff tooling only; they do not drive Blender or Unreal for you.

If you need one realistic end-to-end demo for an operator review, prefer the repo-analysis kit and run:

```bash
cd rust
cargo build --workspace --locked
./scripts/run-repo-analysis-demo.sh
```

That wrapper captures outputs into `.demo-artifacts/repo-analysis-demo/` so the review loop is less ephemeral, and it now stages `bundle-summary.json`, `operator-handoff.json`, `review-status.json`, `review-log.md`, `bundle-checksums.txt`, and `operator-dashboard.html` plus a cross-run `index.json` / `index.html` for bounded review/resume continuity.

## Minimal operator checklist

Use this sequence for a new machine:

```bash
cd rust
cargo build --workspace --locked
./target/debug/claw --help
./target/debug/claw status
export ANTHROPIC_API_KEY="sk-ant-..."   # or: ./target/debug/claw login
./target/debug/claw --corpus ./docs --profile balanced prompt "What does the bootstrap flow do?"
ls -R .claw/trace .claw/telemetry 2>/dev/null
```

If all of that works, you have the main install / auth / corpus / run / inspect loop working.