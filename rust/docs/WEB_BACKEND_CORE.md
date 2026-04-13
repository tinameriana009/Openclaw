# Web backend core

This repo now includes a bounded local backend foundation under `crates/web-backend-core`.

## Honest scope

This is **not** a claim that the project already has a complete live web app.

What exists now is smaller and safer:
- a localhost-only daemon (`claw-webd`)
- persisted service state under `.claw/backend/`
- a small JSON API for queue/state inspection
- a bounded queue mutation slice that is now **deny-by-default** unless an explicit local-only auth-boundary policy file opts in
- a runtime-bridge file intended to mirror or extend the existing static operator artifacts over time

It is still local plumbing for one workspace, not a full multi-user control plane.

## Why this exists

The repo already had strong static/on-disk operator surfaces:
- review bundles
- inbox/handoff concepts
- dashboard/runtime bridge artifacts

But it did **not** yet have a real service-oriented backend layer.

This crate is the first bounded step toward that: a local API and persistence substrate future web-agent work can build on.

## Storage model

By default the daemon uses:

- `.claw/backend/operator-queue.json`
- `.claw/backend/runtime-bridge.json`
- `.claw/backend/operator-inbox.json`

The queue file is the mutable operator work queue.
The runtime bridge file is the local snapshot surface the service exposes to web/API consumers.

## API

- `GET /healthz`
- `GET /v1/schema`
- `GET /v1/state`
- `GET /v1/queue`
- `GET /v1/operator/inbox`
- `POST /v1/operator/inbox/sync`
- `GET /v1/operator/repo-analysis`
- `POST /v1/operator/repo-analysis/sync`
- `POST /v1/queue/items`
- `POST /v1/queue/items/:id/claim`
- `POST /v1/queue/items/:id/unclaim`
- `POST /v1/queue/items/:id/defer`
- `POST /v1/queue/items/:id/complete`
- `POST /v1/queue/items/:id/drop`
- `GET /v1/operator/inbox`
- `POST /v1/operator/inbox/sync`

Example create request:

```json
{
  "title": "Review approval packet",
  "kind": "review",
  "source_path": ".claw/web-approvals/index.json",
  "note": "first operator pass"
}
```

Example claim request:

```json
{
  "claimed_by": "operator-a"
}
```

Example bounded mutation request:

```json
{
  "note": "waiting on upstream context"
}
```

## Mutation semantics

Current mutation behavior is intentionally conservative:

- all HTTP mutation routes are blocked by default
- enabling them now requires a local policy file at `.claw/backend/web-operator-auth-policy.json`
- that policy must keep `mutationRoutesEnabled=false` for any future authenticated/live backend assumptions
- the only currently supported opt-in is `localOperatorMutations.enabled=true`
- when enabled, the daemon must still stay loopback-bound and each mutation request must send the configured acknowledgment header (default: `x-claw-local-operator`)

That is enough to support bounded localhost queue testing without pretending the backend already supports cross-user locking, real authentication, permissions, leases, audit streams, or durable workflow orchestration.

## Run locally

```bash
cd rust
cargo run -p web-backend-core --bin claw-webd
```

Optional env vars:

- `CLAW_WORKSPACE_ROOT=/path/to/workspace`
- `CLAW_WEBD_BIND=127.0.0.1:8787`

Optional local-only mutation policy file:

- `.claw/backend/web-operator-auth-policy.json`
- see `config-examples/web-operator-auth-policy.example.json`
- if absent, mutation routes stay disabled

To prove the daemon is consumable without inventing a full live frontend, you can generate a static HTML status page from the JSON API:

```bash
cd rust
cargo run -p web-backend-core --bin claw-webd -- serve
# in another shell
cargo run -p web-backend-core --bin claw-webd -- export-static-status-page \
  --api-base-url http://127.0.0.1:8787 \
  --output ../.claw/backend/static-status.html
```

That consumer command fetches `/v1/state` and writes a bounded local page summarizing service info, runtime-bridge state, recent traces, and queue items.

## Intended next steps

There is now also a bounded local polling helper for artifact refreshes:

```bash
cd rust
cargo run -p web-backend-core --bin claw-webd -- watch-local-artifacts --once
# or keep polling for local changes
cargo run -p web-backend-core --bin claw-webd -- watch-local-artifacts --poll-interval-ms 2000
```

That command does **not** claim a real-time collaborative backend. It only watches local staged artifacts and refreshes the cached backend bridge/inbox when those source files become newer than the cached backend copies.

Future work can build on this by:
- projecting existing review/handoff/dashboard artifacts into the runtime bridge automatically
- adding bounded status transitions beyond the current local mutation slice if they are truly needed
- wiring an actual web UI to the local JSON API
- replacing polling with a tighter watcher or event stream only if that remains honest and justified

For now, the foundation is intentionally small, local, and explicit.
