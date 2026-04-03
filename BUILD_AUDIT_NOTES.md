# Build / Test Audit Notes

## Status
Preliminary audit

## Date
2026-04-03 UTC

## Scope
Attempted to run baseline Rust workspace verification for `claw-code-parity`.

## Commands attempted
- `cargo build --workspace`
- `cargo test --workspace`

## Initial result
Initial verification failed because the environment did not have the required Rust toolchain available in PATH.

Observed error:
- `/usr/bin/bash: line 1: cargo: command not found`

A second blocker then appeared after installing Ubuntu's packaged Rust toolchain:
- `Cargo.lock` version 4 was not supported by `cargo 1.75.0`

## Resolution
Installed a newer Rust toolchain via `rustup`.

Verified versions:
- `cargo 1.94.1`
- `rustc 1.94.1`

## Final verification result
After upgrading the toolchain:
- `cargo build --workspace` ✅ passed
- `cargo test --workspace` ✅ passed

## Investigation note
During verification, one test initially failed in plugin hook execution:
- `hooks::tests::collects_and_runs_hooks_from_enabled_plugins`

Root cause:
- hook helper code wrote JSON payloads to child stdin with `write_all()`
- simple hook scripts exited without reading stdin
- that caused `BrokenPipe (os error 32)`

Fix applied:
- treat `BrokenPipe` while writing hook stdin as non-fatal in:
  - `rust/crates/plugins/src/hooks.rs`
  - `rust/crates/runtime/src/hooks.rs`

Rationale:
- hooks are allowed to ignore stdin payloads
- a fast-exiting hook should not be treated as a failed start merely because it did not consume stdin

## What was completed
- Static repository audit of Rust workspace layout
- Installed working modern Rust toolchain
- Initial config-surface implementation for:
  - `rag`
  - `rlm`
  - `webResearch`
- Added typed config parsing and tests in `rust/crates/runtime/src/config.rs`
- Re-exported the new config types from `rust/crates/runtime/src/lib.rs`
- Fixed hook runner broken-pipe behavior
- Verified full workspace build and test success

## Notes
There is also a host-level notice about a newer kernel being available (`6.8.0-106-generic` expected vs current `6.8.0-101-generic`), but that did not block repository build/test verification.
