# Rust bootstrap notes

This is the shortest path to a usable local `claw` operator setup.

## 0) Toolchain gotcha first

This repo uses a lockfile/toolchain combination that is newer than the default Cargo on some hosts.
If you see an error like:

- `lock file version 4 requires -Znext-lockfile-bump`

then your shell is picking up an old Cargo.

Use the pinned toolchain from `rust-toolchain.toml`:

```bash
curl https://sh.rustup.rs -sSf | sh
. "$HOME/.cargo/env"
rustup toolchain install 1.94.1 --profile minimal --component clippy --component rustfmt
cd rust
rustup override set 1.94.1
cargo --version
rustc --version
```

If `/usr/bin/cargo` still wins on `PATH`, call `~/.cargo/bin/cargo` directly.

## 1) Build the workspace

```bash
cd rust
cargo build --workspace --locked
```

## 2) Smoke-test the local binary

```bash
./target/debug/claw --help
./target/debug/claw status
./target/debug/claw sandbox
```

`status` is the best first-run check because it confirms:

- model resolution
- permission mode
- active profile
- cwd / project root detection
- git visibility
- config and memory discovery

## 3) Authenticate

### API key

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

### Or OAuth

```bash
./target/debug/claw login
```

Current login behavior:

- listens on `http://localhost:4545/callback`
- prints a Claude OAuth URL
- attempts to open a browser
- falls back to manual open if no browser opener exists

## 4) Run something simple

```bash
./target/debug/claw prompt "summarize this repo"
./target/debug/claw "summarize crates/runtime/src/config.rs"
```

## 5) Attach a local corpus

Attach one or more roots up front:

```bash
./target/debug/claw --corpus ./docs --corpus ./crates/runtime
```

Or start the REPL and use slash commands:

```bash
./target/debug/claw
```

Then:

```text
/corpus
/corpus attach ./docs
/corpus search bootstrap
/corpus inspect <corpus-id>
/corpus slice <chunk-id>
/corpus answer what does the bootstrap doc say about cargo?
```

## 6) Inspect traces

Trace-enabled runs write artifacts under `.claw/`:

- `.claw/trace/`
- `.claw/telemetry/recursive-runtime.jsonl`
- `.claw/sessions/`

Recommended path:

1. use `--profile balanced`, `deep`, or `research`
2. run your prompt or grounded corpus query
3. inspect stdout for the trace summary
4. open the saved ledger from `.claw/trace/`

The REPL also exposes:

```text
/trace summary <trace-file>
/trace export <trace-file> [destination]
```

## Minimal verification suite

Run this before claiming a machine is ready:

```bash
cd rust
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked
cargo test --workspace --locked
./target/debug/claw --help
./target/debug/claw status
```

## Safer first-run defaults

- Default permission mode is `workspace-write` unless config or env overrides it.
- Use `--dangerously-skip-permissions` only when you intentionally want unrestricted execution for that run.
- Saved trace files on disk are currently the most reliable trace-inspection surface for operators.

## One command sequence for a new machine

```bash
cd rust && \
  cargo build --workspace --locked && \
  ./target/debug/claw --help && \
  ./target/debug/claw status
```

Then authenticate and continue with corpus / prompt flows.