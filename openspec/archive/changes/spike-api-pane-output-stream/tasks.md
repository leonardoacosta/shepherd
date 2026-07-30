# Tasks — spike-api-pane-output-stream (design spike)

Base commit: `1de05dc2`. This is investigation + design, not a build. Read-heavy.

Exemplar: `src/api/subscriptions.rs` (existing event fan-out),
`src/server/client_transport.rs` (the capacity-1 `render` slot as the
backpressure precedent), `src/api/schema/events.rs` (event-shape conventions).

## Steps

1. **Map the current output paths.** Document how `pane.read` and
   `pane.wait_for_output` work today and where live pane bytes exist server-side
   before rendering.

2. **Answer the four design questions** (event shape, backpressure, compat
   surface, fan-out) in a design doc under `docs/next/` or `.local/prd/`. Give a
   defended recommendation for each.

3. **Prototype or schema.** Either implement a `pane.output` subscription behind
   an experimental/undocumented flag (bounded buffer, gap markers), or write the
   `src/api/schema/` addition + backpressure spec without wiring it live.
   - Gate (if code lands): `just check` passes; a manual `herdr` session shows a
     subscriber receiving tailed output with a bounded buffer.

4. **Hand off.** done-when: proposal `spike-api-pane-output-stream` archived with
   the design doc merged and a follow-up build proposal (or an explicit "defer"
   determination) recorded.

## STOP / decision points

- If the maintainer decides the private frame protocol should stay the only
  output surface for now, record that as the determination and archive — a "no"
  with reasons is a valid spike outcome.
