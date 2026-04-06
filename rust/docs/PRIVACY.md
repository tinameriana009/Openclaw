# Privacy and operator handling notes

The Rust harness can write useful local artifacts under `.claw/`. Those artifacts are operationally valuable, but they may also contain sensitive data.

## What may be sensitive

Depending on the task and workspace, saved artifacts can reveal:

- local file paths
- document titles and headings
- chunk previews from attached corpora
- prompts and task descriptions
- model outputs and trace/event metadata
- session history
- repository names, branch names, and execution context

## Default operator stance

Treat `.claw/` as potentially sensitive local state.

Recommended handling:

- keep `.claw/` local by default
- review trace/session files before sharing them externally
- scrub paths, identifiers, and content previews when posting bug reports
- avoid attaching raw corpus manifests to public issues unless the corpus is already public

## What traces are for

Saved traces are intended to improve trust:

- they show what the runtime did
- they show retrieval/subquery/web-escalation structure
- they do **not** promise a safe-to-share artifact by default

The safest sharing pattern is:

1. start from the saved trace ledger
2. redact path/content identifiers as needed
3. include the workspace version / commit
4. attach a short reproduction note instead of a whole session dump when possible

## Telemetry file handling

`recursive-runtime.jsonl` should be treated like a local debug log, not anonymous analytics. Even if individual events are compact, aggregate logs can still reveal behavior and repository context.

## Corpus artifact handling

Corpus manifests can expose:

- root directories
- document paths
- headings
- chunk previews

That makes them especially sensitive on private repos or note collections.

## Recommended bug-report bundle

If you need to report a trace/runtime issue to another maintainer, prefer this bundle order:

- commit hash or release tag
- exact command/profile used
- redacted trace ledger excerpt
- relevant stderr/stdout excerpt
- only the minimum corpus/session details needed to reproduce

## Not promised yet

This repo does **not** currently claim:

- built-in redaction tooling
- automatic PII stripping
- encrypted local artifact storage
- remote telemetry privacy guarantees

So the right operating assumption today is simple: **saved local artifacts are for informed operators, not for blind sharing.**
