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

Preferred one-shot path:

```bash
cd rust
./scripts/release-verify.sh
```

For stricter RC discipline, use:

```bash
cd rust
RELEASE_CANDIDATE=1 ./scripts/release-verify.sh
```

That RC mode keeps the same locked build/test gates, requires a clean working tree, runs `python3 ../tests/validate_release_candidate_readiness.py`, and emits a machine-readable release artifact manifest under `.claw/release-artifacts/release-manifest.json` that is immediately re-validated against the current binary/docs. That keeps the RC claim tied to current docs/trust notes and concrete artifact hashes rather than memory alone. See [`docs/RELEASE_CANDIDATE.md`](docs/RELEASE_CANDIDATE.md) for the bounded RC flow.

Manual equivalent:

```bash
cd rust
cargo build --workspace --locked
cargo fmt --all --check
cargo clippy --workspace --all-targets --locked
cargo test --workspace --locked
./target/debug/claw --help
./target/debug/claw status
python3 ../tests/validate_operator_readiness.py
python3 ../tests/validate_blender_demo.py
python3 ../tests/validate_unreal_demo.py
python3 ../tests/validate_repo_analysis_demo.py
python3 ../tests/validate_release_candidate_readiness.py  # when running an RC gate
manifest_path=$(./scripts/generate-release-artifact-manifest.sh)
python3 ../tests/validate_release_artifact_manifest.py "$manifest_path"
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
- `python3 ../tests/validate_operator_readiness.py` passes so release docs and workflow honesty cues stay aligned
- demo validators pass and do not leave a surprising dirty tree (generated demo zips are ignored or cleaned)

## Release candidate discipline

Before calling something `rc`, explicitly check these:

- `CHANGELOG.md`, `README.md`, `BOOTSTRAP.md`, `docs/ARTIFACTS.md`, and `docs/PRIVACY.md` describe the current behavior rather than the intended future behavior
- artifact compatibility notes are present in the release draft, even if the note is simply "no schema change from prior RC"
- trace/corpus artifacts from the current build include `artifactKind`, `schemaVersion`, and `compatVersion`
- older local `.claw/` state is treated as upgrade-sensitive local state, not as a guaranteed cross-version interchange contract
- if you changed artifact fields, you either documented the migration impact or told operators to rebuild local corpora / re-run with fresh traces

A practical release-candidate rule for this repo:

- **alpha smoke pass:** `./scripts/release-verify.sh`
- **release candidate pass:** `RELEASE_CANDIDATE=1 ./scripts/release-verify.sh`
- **taggable RC:** clean tree, current docs, explicit compatibility note, and at least one grounded operator run on a fresh `.claw/` state

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
- Did any `.claw/corpora/` manifest keys change?
- Did session resume behavior change?
- Did any command/help surface or profile default change?
- Do operators need to delete old local artifacts or rebuild corpora?

If the answer is "no", say so plainly in the release notes.

## Current migration baseline

As of `0.1.0`:

- trace ledgers and corpus manifests now include `artifactKind`, `schemaVersion`, and `compatVersion`
- legacy unversioned trace/corpus artifacts are still read defensively by the current runtime
- no dedicated artifact migration layer exists yet beyond backward-compatible readers
- safest automation strategy is still pinning to a tag/commit and parsing defensively
- old `.claw/` directories should still be treated as local state, not as guaranteed cross-version interchange

## Trust signals worth preserving

Every release should keep these current:

- `rust/README.md`
- `rust/BOOTSTRAP.md`
- `rust/CHANGELOG.md`
- `rust/docs/ARTIFACTS.md`
- `rust/docs/PRIVACY.md`

If those drift from reality, operator trust drops faster than code quality improves.
