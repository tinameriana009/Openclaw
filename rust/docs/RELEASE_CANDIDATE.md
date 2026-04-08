# Release-candidate discipline

This is the practical RC gate for the current source-first Rust harness.

Use it when you want a stronger claim than "the workspace builds and tests": it should tell you whether the repo is release-candidate clean enough to tag or circulate as an RC without pretending the project is more polished than it is.

## RC entrypoint

```bash
cd rust
RELEASE_CANDIDATE=1 ./scripts/release-verify.sh
```

That path keeps the locked build/test gates, requires a clean tree, and runs the RC-readiness validator in addition to the normal operator/demo validators.

## What RC mode is actually asserting

An RC pass here means all of the following are true:

- the working tree is clean
- the pinned toolchain is active
- the locked workspace verification still passes
- the operator/trust docs exist and still point at the current release flow
- the release docs still carry explicit compatibility/migration language
- the artifact trust story still mentions `artifactKind`, `schemaVersion`, and `compatVersion`
- a fresh machine-readable release manifest can be generated and re-validated against the current binary/docs
- a paired `release-attestation.json` can be generated and validated as a formal local provenance statement
- if you provide `PROVENANCE_SIGNING_KEY`, a signed `release-provenance.json` + `.sig` bundle can be generated and validated as a bounded cryptographic chain over the binary/manifest/attestation set

It does **not** mean every operator workflow is fully automated, every artifact is a permanent interchange contract, or the broader product is production-final.

## Triage checklist before calling something `rc`

1. Run `RELEASE_CANDIDATE=1 ./scripts/release-verify.sh`.
2. Confirm the release notes draft includes an **Operator notes** section.
3. Fill in **Compatibility or migration notes** explicitly, even if the answer is: `no schema change from prior RC`.
4. If trace or corpus fields changed, explain the operator impact plainly.
5. Do one grounded run against a fresh `.claw/` state before tagging.
6. Generate and validate `.claw/release-artifacts/release-manifest.json` and `.claw/release-artifacts/release-attestation.json` so the RC notes can point at exact current hashes/bytes for the binary and trust docs plus a formal statement envelope.
7. If you maintain a release key, set `PROVENANCE_SIGNING_KEY` and generate `.claw/release-artifacts/release-provenance.json` plus `.sig` to publish a bounded signed provenance chain instead of unsigned local attestation only.

## Fresh-state rule

The safest RC posture is testing with a fresh local artifact directory once per candidate:

```bash
cd rust
mv .claw .claw.pre-rc-backup 2>/dev/null || true
./target/debug/claw --corpus ./docs --profile balanced prompt "What does the bootstrap flow do?"
ls -R .claw/trace .claw/telemetry .claw/corpora 2>/dev/null
```

Why this matters:

- older local `.claw/` state is still treated as upgrade-sensitive local state
- backward-compatible readers exist, but there is no dedicated migration layer yet
- an RC should not rely only on stale local artifacts from previous iterations

## Release-note minimum

Before tagging, the draft release notes should answer:

- Build/install: anything operators must do differently?
- Auth: anything changed in login/logout expectations?
- Corpus / trace behavior: anything new or materially different?
- Compatibility or migration notes: do operators need to rebuild corpora, discard traces, or treat older artifacts with caution?

If the answer is "nothing changed," say that plainly instead of leaving the section blank.

## Trust boundary reminder

A passing RC gate improves trust; it does not remove the current pre-1.0 caution:

- pin automation to a tag or commit
- parse `.claw/` artifacts defensively
- treat shared traces/manifests as potentially sensitive
- keep operator language honest about what is manual versus fully productized
