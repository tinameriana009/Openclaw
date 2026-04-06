# Changelog

All notable operator-facing changes in the Rust harness should be recorded here.

This project is still pre-1.0. Until a packaged release flow exists, treat entries here as the canonical release-notes scaffold for source builds and tagged snapshots.

The format is intentionally lightweight and loosely follows Keep a Changelog.

## [Unreleased]

### Added
- Explicit release-hygiene guidance for clean-tree checks, branch/remote posture, and final verification before tagging.
- `rust/scripts/release-verify.sh` to run the exact locked release verification sequence with an upfront Rust toolchain sanity check.

### Changed
- Top-level repository docs now tell operators to verify the canonical publishing remote with `git remote -v` instead of assuming a stale GitHub URL.
- Rust bootstrap/readme/release docs now point to the release verification helper first, with the manual locked command sequence kept inline.

### Fixed
- Rust workspace formatting so the documented `cargo fmt --all --check` release gate passes again.
- A stray bullet formatting typo in `NEXT_ACTIONS.md`.
- Blender demo packaging output under `docs/examples/blender-scene-cleanup-demo/dist/` no longer leaves a noisy untracked release artifact in `git status`.

### Removed
- None yet.

## [0.1.0] - 2026-04-06

Initial documented Rust harness baseline for the current repo state.

### Ships
- Interactive REPL and non-interactive prompt mode.
- Session autosave and resume.
- Permission / sandbox controls.
- Local corpus attach, inspect, search, slice, and answer surfaces.
- Recursive trace ledger export and runtime telemetry.
- Execution profiles: `fast`, `balanced`, `deep`, `research`.

### Operator notes
- Build/install remains source-first.
- Saved trace files under `.claw/trace/` remain the safest trace-inspection surface.
- Artifact JSON formats are stable enough for operator inspection, but should still be treated as pre-1.0 contracts unless explicitly versioned in a future release.
