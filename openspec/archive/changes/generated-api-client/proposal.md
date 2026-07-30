# Generate a typed API client and reference consumer from the schema

Base commit: `1de05dc2` · Route: proposal (feature) · Effort: M · Confidence: MED (greenfield deliverable; depends on the output-stream direction landing) · Category: direction / DX

## Why

Adjacent follow-on to `spike-api-pane-output-stream` (DIRECTION-01). Once the
output stream completes the API surface, a generated client is "one file away"
and is what proves the "TUI as one client" thesis with a demonstrable second
client — the web-UI / mobile-attach direction, and the sponsor pitch's "real
agent runtime".

The machine-readable contract already exists and is CI-protected:

- `docs/next/api/herdr-api.schema.json` — the JSON schema, generated via
  `schemars` derives on the `Method` enum and every params/response type
  (`src/api/schema.rs`).
- `src/api/schema/tests.rs:~159` asserts the committed schema matches the code;
  `tests/cli/surface.rs:~538` covers `herdr api schema`.

But there is no client library: every integrator hand-rolls request framing,
method names, and param shapes against the raw socket. The schema makes a typed
client generatable rather than hand-written, and CI-checkable to stay in sync.

## What changes

1. A generator that emits typed bindings from `herdr-api.schema.json` for at least
   one target (TypeScript recommended — the web-UI direction; Python is the agent-
   scripting alternative). Prefer a schema-to-types tool over bespoke codegen.
2. A thin reference client wrapping socket connect + request framing + the
   `events.subscribe` loop, so a consumer calls `client.paneList()` not raw JSON.
3. A CI check that regenerates bindings and fails if they drift from the committed
   schema (mirroring how `src/api/schema/tests.rs` pins the schema itself).

## Acceptance

- Generated bindings exist for the chosen target, checked in with a regenerate
  command documented.
- A reference client can, against a running `herdr`, list panes and subscribe to
  events end-to-end (a smoke test or example script).
- A CI check fails when the schema changes without regenerating bindings.
- `just check` passes (any Rust-side changes, e.g. a `herdr api schema` output
  tweak, stay green).

## Out of scope

- A full web UI or mobile app — this delivers the client layer they'd build on,
  not the app.
- Publishing the client to a package registry — decide separately; in-repo +
  example is enough to prove the surface.

## Dependencies

- Best sequenced **after** `spike-api-pane-output-stream` decides the output-
  stream shape, so the client covers the completed surface rather than needing a
  breaking regen later. Can start against today's surface if the maintainer wants
  it sooner — note the output stream as a follow-up addition.

## STOP conditions

- If the schema omits types the socket actually uses (e.g. `#[schemars(skip)]`
  streaming variants a real client needs), STOP and report the gap — the client
  can't be complete against an incomplete schema, and the fix is on the schema
  side.
