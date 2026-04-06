# Changelog

All notable operator-facing changes in the Rust harness should be recorded here.

This project is still pre-1.0. Until a packaged release flow exists, treat entries here as the canonical release-notes scaffold for source builds and tagged snapshots.

The format is intentionally lightweight and loosely follows Keep a Changelog.

## [Unreleased]

### Added
- Release-note scaffolding and operator trust docs.
- Artifact contract notes for trace, telemetry, session, and corpus outputs.
- Privacy / handling guidance for saved `.claw/` artifacts.

### Changed
- None yet.

### Fixed
- None yet.

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
