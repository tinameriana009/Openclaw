# Rust bootstrap notes

This repo currently needs a newer Cargo than the system default on some hosts.

## Known toolchain gotcha

`Cargo.lock` is lockfile v4. If `cargo test` fails with:

- `lock file version 4 requires -Znext-lockfile-bump`

then you're hitting an older Cargo on `PATH`.

Use the newer Cargo binary instead:

```bash
~/.cargo/bin/cargo --version
~/.cargo/bin/cargo test
~/.cargo/bin/cargo build
```

On the OpenClaw audit host, `/usr/bin/cargo` was too old while `~/.cargo/bin/cargo` worked.

## Minimal local smoke test

```bash
cd rust
~/.cargo/bin/cargo test -p runtime
~/.cargo/bin/cargo test -p rusty-claude-cli
./target/debug/claw --help
./target/debug/claw status
./target/debug/claw --corpus . /corpus
```

## Safer first run

The CLI now defaults to `workspace-write` when neither config nor env overrides it.
Use `--dangerously-skip-permissions` only when you really want unrestricted tool execution.

## Grounded corpus flow

You can attach a local corpus at startup and then exercise the recursive grounded path:

```bash
./target/debug/claw --corpus docs
# then inside the REPL:
/corpus
/corpus search bootstrap
/corpus answer what does the bootstrap doc say about cargo?
```

That path writes trace artifacts under `.claw/trace/` and telemetry under `.claw/telemetry/`.
