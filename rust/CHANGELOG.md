# Changelog

All notable operator-facing changes in the Rust harness should be recorded here.

This project is still pre-1.0. Until a packaged release flow exists, treat entries here as the canonical release-notes scaffold for source builds and tagged snapshots.

The format is intentionally lightweight and loosely follows Keep a Changelog.

## [Unreleased]

### Added
- Explicit artifact envelope metadata for new trace ledgers and corpus manifests: `artifactKind`, `schemaVersion`, and `compatVersion`.
- Backward-compatible readers and regression tests so current builds still accept older unversioned trace/corpus artifacts.
- Release-candidate discipline in `rust/scripts/release-verify.sh` via `RELEASE_CANDIDATE=1`, including clean-tree enforcement and explicit RC reminders.
- Stronger artifact trust/privacy notes covering compatibility anchors and redaction-safe sharing.

### Changed
- `rust/RELEASE.md` now distinguishes ordinary verification from stricter RC verification and documents the current migration baseline more honestly.
- `rust/docs/ARTIFACTS.md` now documents the artifact envelope and the recommended parsing strategy for pre-1.0 automation.
- `rust/docs/PRIVACY.md` now recommends preserving only envelope metadata while redacting sensitive path/content details from shared bug reports.

### Fixed
- Release/readiness guidance now points at the actual corpus storage path `.claw/corpora/` instead of the older singular form.
- Trace/corpus compatibility notes now reflect implemented version markers instead of describing them only as future work.

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
