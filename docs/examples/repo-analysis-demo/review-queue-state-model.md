# Review queue state model: repo analysis demo

This demo now stages a bounded operator queue model so the workflow feels closer to a real review queue without pretending a live service exists.

## Scope

This is a **static on-disk state model** for staged demo bundles.
It is not a daemon, not a browser-backed queue, and not a concurrency-safe multi-user coordination system.

## Queue lifecycle

Each staged run carries `queue-state.json` with these bounded lifecycle fields:

- `lifecycleState`
  - `queued` — new run exists but nobody has started review yet
  - `claimed` — an operator has taken ownership, but review work has not really started
  - `in-review` — evidence gathering or trace review is underway
  - `deferred` — review was intentionally paused and should be revisited later
  - `handoff-ready` — current operator finished a bounded pass and left a clean handoff
  - `completed` — review is done for this bundle
  - `dropped` — bundle will not be pursued further

- `queueAction`
  - `claim`
  - `ack`
  - `defer`
  - `handoff`
  - `complete`
  - `drop`

## Queue metadata

The staged file also carries honest queue-style metadata:

- `priority`: `low` / `normal` / `high`
- `claimedBy`
- `claimedAtUtc`
- `acknowledgedAtUtc`
- `deferredUntilUtc`
- `deferReason`
- `handoffTarget`
- `handoffSummary`
- `lastUpdatedAtUtc`
- `reviewAgeMinutes`
- `queueLane`
- `artifacts`
- `continuityCommands`

## Operator intent

The point is to support a more realistic flow:

1. a run lands in `queued`
2. someone `claim`s it
3. they `ack` and move it to `in-review`
4. they either `defer`, mark it `handoff-ready`, or `complete` it

That makes the static dashboard/index feel more like a real operator system while staying honest about current limits.

## Important limitation

Editing `queue-state.json`, `review-status.json`, and `review-log.md` is still manual.
There is no live broker, lease timeout, push update bus, or browser automation layer here today.
