# Release and readiness checklist

This project is still source-first, but releases are easier to trust when they follow a repeatable checklist.

## Before tagging or announcing

### Repository hygiene / release posture

```bash
git status --short
git branch -vv
git remote -v
```

Confirm all of the following before you cut a tag or write release notes:

- the working tree is clean
- you know which remote is canonical for this clone
- the branch you are about to tag/publish is pushed where you expect
- local-only remotes or unpublished commits are called out explicitly if they still exist

### Build / verification

```bash
cd rust
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked
cargo test --workspace --locked
cargo build --workspace --locked
./target/debug/claw --help
./target/debug/claw status
```

### Operator sanity checks

Run at least one end-to-end grounded flow:

```bash
./target/debug/claw --corpus ./docs --profile balanced prompt "What does the bootstrap flow do?"
ls -R .claw/trace .claw/telemetry 2>/dev/null
```

Confirm:

- help and status surfaces work
- auth story is still accurate in docs
- trace files are written when expected
- telemetry file is written when expected
- corpus attach/search/inspect flow still works

## Release notes template

Copy into the top of `CHANGELOG.md` or a GitHub release draft.

```md
## [X.Y.Z] - YYYY-MM-DD

### Added
- 

### Changed
- 

### Fixed
- 

### Removed
- 

### Operator notes
- Build/install:
- Auth:
- Corpus / trace behavior:
- Compatibility or migration notes:
```

## Compatibility / migration section to include

For every release, answer these explicitly:

- Did any `.claw/trace/` artifact keys change?
- Did any `.claw/corpus/` manifest keys change?
- Did session resume behavior change?
- Did any command/help surface or profile default change?
- Do operators need to delete old local artifacts or rebuild corpora?

If the answer is "no", say so plainly in the release notes.

## Current migration baseline

As of `0.1.0`:

- no dedicated artifact migration layer exists
- no explicit schema-version fields exist on trace/corpus artifacts
- safest automation strategy is pinning to a tag/commit and parsing defensively
- old `.claw/` directories should be treated as local state, not as guaranteed cross-version interchange

## Trust signals worth preserving

Every release should keep these current:

- `rust/README.md`
- `rust/BOOTSTRAP.md`
- `rust/CHANGELOG.md`
- `rust/docs/ARTIFACTS.md`
- `rust/docs/PRIVACY.md`

If those drift from reality, operator trust drops faster than code quality improves.
