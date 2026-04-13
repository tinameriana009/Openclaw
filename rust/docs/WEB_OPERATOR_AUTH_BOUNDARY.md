# Web operator auth boundary (bounded scaffold)

This repository does **not** currently ship a real authenticated web operator backend.

Today, the honest state is narrower:

- the current operator-facing web-shaped surfaces are static on-disk artifacts
- trace approval/replay/resume flows are still operator-driven CLI flows
- there is no supported multi-user session service
- there are no issued operator identities, no password login, no session cookie flow, and no browser-side privilege model to rely on yet

This document is a **guardrail for future work**, not a claim that those pieces already exist.

## Why this exists

The current Rust harness has some browser-shaped review artifacts and bounded web-aware execution semantics. That can make it tempting to bolt on a live backend quickly.

That would be a mistake unless the security boundary is explicit first.

The smallest safe slice today is:

1. say what is **not** implemented
2. define the minimum boundary a future live service must honor
3. provide a machine-checkable policy schema that defaults to **disabled**

## Current truth

If you expose this repo behind a live HTTP service today, you are building your **own** auth and deployment wrapper around a CLI-oriented toolchain.

Treat that wrapper as out-of-tree operator responsibility unless and until the repo grows a first-class web auth implementation.

## Non-goals of the current repo

The current repo does **not** promise:

- password authentication
- built-in operator user management
- RBAC or per-user authorization
- browser session cookies
- CSRF protection for a live app
- API token issuance or revocation
- SSO/OIDC/SAML integration
- audited multi-tenant isolation
- hardened secret storage for a web service
- internet-safe default exposure

## Minimum boundary for any future live web operator backend

Any future live backend should start from these constraints:

### 1. Disabled by default

A live web operator backend must be considered **off by default**.

If a future config includes an auth policy, the safe default is:

- backend disabled
- local/static/operator-only assumptions retained
- no anonymous mutation endpoints

### 2. Reverse-proxy auth first

Before there is a real in-process auth system, the only acceptable near-term deployment assumption is:

- bind locally or to a trusted private network only
- place any live HTTP surface behind an authenticated reverse proxy or equivalent trusted gateway
- pass a small, explicit identity envelope from that proxy to the backend
- reject requests when that envelope is absent or malformed

In other words: **no direct internet exposure, no anonymous operator panel, no fake “temporary auth” query params.**

### 3. Read-only before write-capable

If a future web surface appears, it should begin as read-only/inspection-oriented first.

High-risk actions such as these should stay CLI/operator-confirmed until there is real authn/authz, auditing, and anti-CSRF/session handling:

- running tools or shell commands
- approving web escalations
- changing runtime config
- mutating session state across operators
- exporting traces/corpora containing sensitive content

### 4. Explicit trust boundary headers only

If a reverse proxy identity envelope is used before first-class auth exists, it should be explicit and narrow, for example:

- operator identifier
- display name
- auth mechanism/source
- request id
- group/role list if present

The backend should not trust arbitrary client-supplied impersonation headers from the open internet. Only a trusted proxy layer should be allowed to inject them.

### 5. Audit trail required for mutation

Any future authenticated mutation route should eventually write an operator-attributed audit trail recording at least:

- who initiated it
- what action was attempted
- when it happened
- whether it succeeded
- which local artifacts/session ids were affected

This repo does not implement that audit layer yet, so mutation-capable web routes should be treated as out of bounds.

## Policy schema

A conservative machine-readable policy schema lives at:

- [`docs/schemas/web-operator-auth-policy.schema.json`](schemas/web-operator-auth-policy.schema.json)

An example deny-by-default policy lives at:

- [`config-examples/web-operator-auth-policy.example.json`](../config-examples/web-operator-auth-policy.example.json)

The schema is intentionally strict and conservative:

- `backendEnabled` defaults to `false`
- `directInternetExposureAllowed` must remain `false`
- `anonymousReadAllowed` must remain `false`
- `mutationRoutesEnabled` defaults to `false`
- `trustedProxy.required` defaults to `true`
- `sessionCookiesSupported` must remain `false`
- `localOperatorMutations.enabled` defaults to `false`

A small bounded exception now exists for the in-tree localhost queue mutation slice:

- it is still **not** real auth
- it requires an explicit policy file opt-in
- it can require loopback binding
- it requires a per-request acknowledgment header (default `x-claw-local-operator`)

That exception is meant to keep the existing local backend honest while still making any mutation behavior pass through an explicit boundary contract.

That schema is a **boundary contract**, not an implementation contract.

## Deployment guidance right now

If someone insists on a web wrapper before a real backend exists, the least-bad path is:

1. keep the actual harness private
2. expose only static/generated review artifacts when possible
3. require authenticated reverse-proxy access for any dynamic endpoint
4. keep dynamic endpoints read-only
5. block public exposure by default
6. document the wrapper as deployment-specific and unsupported by the repo itself

## What would count as “real auth” later

Future work could replace this scaffold with a real implementation only when the repo grows at least some combination of:

- authenticated operator identity establishment
- authorization boundaries for sensitive actions
- CSRF/session protections if browser sessions exist
- server-side audit logging for privileged actions
- explicit secret handling rules
- tested deployment guidance for local vs private-network vs internet-exposed modes

Until then, this file exists to keep the line clear: **web-shaped does not mean web-safe.**
