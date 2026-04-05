# Claw Code Rust Workspace

This `rust/` tree is the current Rust harness implementation for `claw`.

It already covers the core interactive CLI loop, tool execution, session persistence, config loading, corpus attachment/search, recursive runtime scaffolding, trace ledgers, and M5 UX/reporting polish. If you are trying to run or evaluate the parity work, start here rather than the legacy top-level repository story.

## What ships today

- Interactive REPL with slash commands, session autosave, resume, export, cost/status views
- Non-interactive prompt mode (`claw prompt ...` or shorthand `claw "..."`)
- Permission modes, allowed-tool filtering, and sandbox status inspection
- Local corpus attach/search/slice/inspect flows
- Recursive corpus answer path with trace + telemetry artifacts
- Execution profiles (`fast`, `balanced`, `deep`, `research`) that change RAG/RLM/web budgets
- Final-answer rendering with sources/confidence/trace id support
- Regression tests for CLI flags, resume flows, final-answer fixtures, and trace fixtures

## Install / build

### Toolchain gotcha

This workspace uses a modern lockfile/toolchain. On some Ubuntu hosts the packaged `cargo` is too old and fails with `Cargo.lock version 4 was not supported`.

If that happens, use `rustup` and set a default toolchain:

```bash
curl https://sh.rustup.rs -sSf | sh
. "$HOME/.cargo/env"
rustup default stable
cargo --version
rustc --version
```

The recent successful audit in this repo used a current Rust toolchain installed via `rustup`, not the distro-packaged Cargo.

### Build and test

```bash
cd rust
cargo build --workspace
cargo test --workspace
```

### Run the binary

```bash
cd rust
cargo run -p rusty-claude-cli -- --help
cargo run -p rusty-claude-cli -- status
cargo run -p rusty-claude-cli -- prompt "summarize this repo"
```

After `cargo build`, the binary is available at `rust/target/debug/claw`.

## Authentication

Use either an API key:

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

Or OAuth:

```bash
cargo run -p rusty-claude-cli -- login
```

## Quick usage

### Interactive REPL

```bash
cargo run -p rusty-claude-cli --
```

### One-shot prompt

```bash
cargo run -p rusty-claude-cli -- prompt "explain the session model"
cargo run -p rusty-claude-cli -- "summarize the runtime crate"
```

### Model / permission / profile selection

```bash
cargo run -p rusty-claude-cli -- --model sonnet --permission-mode read-only status
cargo run -p rusty-claude-cli -- --profile deep prompt "trace the recursive answer path"
```

### JSON output for automation

```bash
cargo run -p rusty-claude-cli -- --output-format json prompt "summarize src/main.rs"
```

## Execution profiles

Profiles tune how much recursive/runtime machinery is enabled:

| Profile | Intended use | Notes |
|---|---|---|
| `fast` | Cheap/local quick answers | RLM disabled, no trace capture |
| `balanced` | Default everyday mode | Recursive path enabled, trace on |
| `deep` | Heavier local investigation | More depth/subcalls, trace on |
| `research` | Maximum recursive/web budget | Largest depth/fetch budget, trace on |

Examples:

```bash
cargo run -p rusty-claude-cli -- --profile fast prompt "what files define config loading?"
cargo run -p rusty-claude-cli -- --profile research prompt "map the corpus + trace pipeline"
```

## Corpus flows

You can attach local files as a corpus up front, or use `/corpus` commands from inside the REPL.

### Attach a corpus before running

```bash
cargo run -p rusty-claude-cli -- --corpus ./docs --corpus ./rust/crates/runtime status
```

### Grounded prompt against attached corpora

```bash
cargo run -p rusty-claude-cli -- --corpus ./docs --profile research prompt "What does the auth flow do?"
```

### REPL corpus commands

```text
/corpus
/corpus attach ./docs
/corpus search auth callback
/corpus slice <chunk-id>
/corpus inspect <corpus-id>
/corpus answer summarize the trace format
```

`/corpus answer ...` is the most important discoverability path here: it runs the grounded recursive corpus flow and emits a trace summary + telemetry location in the result.

## Trace / telemetry artifacts

The recursive corpus answer path writes artifacts under the workspace `.claw/` directory:

- `.claw/trace/` — JSON trace ledgers
- `.claw/telemetry/recursive-runtime.jsonl` — recursive runtime telemetry stream
- `.claw/sessions/` — autosaved sessions

Today, the main user-facing trace flow is:

1. attach one or more corpora
2. run `/corpus answer <query>` or a non-interactive prompt with `--corpus` and a trace-enabled profile
3. inspect the rendered trace summary in stdout
4. open the saved JSON ledger in `.claw/trace/`

Note: the `/trace` slash-command surface exists in the command registry, but REPL/resume handling is still only partially wired in this snapshot. The saved trace artifacts are the reliable path today.

## Useful commands

```bash
cargo run -p rusty-claude-cli -- --help
cargo run -p rusty-claude-cli -- status
cargo run -p rusty-claude-cli -- sandbox
cargo run -p rusty-claude-cli -- --resume latest /status
cargo run -p rusty-claude-cli -- --resume latest /diff /export notes.txt
```

## Workspace layout

```text
rust/
├── Cargo.toml
├── Cargo.lock
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

## Operator notes

- Sessions autosave to `.claw/sessions/<session-id>.jsonl`
- Config is loaded from the standard layered locations exercised by the CLI tests
- `status` is the quickest smoke test for model/permission/profile/config discovery
- `cargo test --workspace` already covers the main M5 fixture/regression surfaces; add more around traces/corpus UX before claiming full parity
