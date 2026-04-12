# Web backend core

This repo now includes a bounded local backend foundation under `crates/web-backend-core`.

## Honest scope

This is **not** a claim that the project already has a complete live web app.

What exists now is smaller and safer:
- a localhost-only daemon (`claw-webd`)
- persisted service state under `.claw/backend/`
- a small JSON API for queue/state inspection and claim flow
- a runtime-bridge file intended to mirror or extend the existing static operator artifacts over time

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

The queue file is the mutable operator work queue.
The runtime bridge file is the local snapshot surface the service exposes to web/API consumers.

## API

- `GET /healthz`
- `GET /v1/schema`
- `GET /v1/state`
- `GET /v1/queue`
- `POST /v1/queue/items`
- `POST /v1/queue/items/:id/claim`

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

## Run locally

```bash
cd rust
cargo run -p web-backend-core --bin claw-webd
```

Optional env vars:

- `CLAW_WORKSPACE_ROOT=/path/to/workspace`
- `CLAW_WEBD_BIND=127.0.0.1:8787`

## Intended next steps

Future work can build on this by:
- projecting existing review/handoff/dashboard artifacts into the runtime bridge automatically
- adding bounded status transitions beyond `claim`
- wiring an actual web UI to the local JSON API
- adding file watchers or event streams if/when justified

For now, the foundation is intentionally small, local, and explicit.
