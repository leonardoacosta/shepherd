# Tasks — generated-api-client

Base commit: `1de05dc2`. Drift check: confirm `docs/next/api/herdr-api.schema.json`
is generated+committed and `herdr api schema` still emits it
(`tests/cli/surface.rs:~538`). If the schema location/shape changed, adjust.

Exemplar: `src/api/schema/tests.rs:~159` (how the committed schema is pinned to
the code) — mirror that "regenerate + assert no drift" discipline for the client
bindings. For an existing JS/TS build+test surface to place the client near, see
`workers/plugin-marketplace/` (bun-based, has `bun test`).

## Ordered steps

1. **Pick target + generator.** Choose TypeScript (recommended) and a
   schema-to-types tool (e.g. json-schema-to-typescript) over bespoke codegen.
   Record the choice.

2. **Generate bindings** from `herdr-api.schema.json` into a new in-repo location
   (e.g. `clients/ts/` or under `workers/`), with a documented regenerate command
   in `justfile`.

3. **Write the reference client**: socket connect, request framing (id + method +
   params), response correlation, and an `events.subscribe` async iterator. Keep
   it thin — it wraps the wire, it does not add semantics.

4. **Smoke test end-to-end.** Against a running `herdr` (or the test-server
   harness in `tests/support/`), list panes and receive at least one event.
   - Gate: the smoke test / example passes.

5. **Add the drift check.** A CI step that regenerates bindings and diffs against
   the committed copy; fail on drift. Wire into the existing bun test surface or a
   new small job.
   - Gate: changing the schema without regenerating makes CI red.

6. **Verify and close.**
   - `just check` passes (Rust side). done-when: proposal `generated-api-client`
     archived.

## STOP conditions

- If no maintained schema-to-types tool handles the schema's constructs (tagged
  enums via `#[serde(tag=...)]`), report before hand-rolling a generator — that
  changes the effort materially and is a maintainer call.
