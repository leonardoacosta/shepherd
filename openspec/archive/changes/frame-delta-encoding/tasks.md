# Tasks — frame-delta-encoding

Base commit: `1de05dc2`. Drift check: confirm `src/server/render_stream.rs:65-112`
still has a `Semantic` arm emitting `ServerMessage::Frame(frame)` whole, and that
`PROTOCOL_VERSION` is at `src/protocol/wire.rs:16`. If the encoding path was
already rewritten, STOP and report.

Exemplar: the `TerminalAnsi` arm of `prepare_frame` (same function) and the blit
encoder in `src/protocol/render_ansi.rs` — the existing row/cell diffing to
imitate for the semantic delta.

## Ordered steps (staged — land step group A first, ship, then B)

### A. FrameDelta behind negotiation
1. Add `ServerMessage::FrameDelta { seq, base_seq, cell_runs }` to
   `src/protocol/wire.rs`; define the run/cell-range encoding. Bump
   `PROTOCOL_VERSION` per AGENTS.md's protocol rule; update hardcoded protocol
   expectations and manual fixtures in tests.
2. In `prepare_frame`'s `Semantic` arm, compute a delta from the retained dirty
   patches (reuse `src/server/headless.rs:3454-3521` bookkeeping) and emit
   `FrameDelta` when a valid base exists, `Frame` as keyframe otherwise.
3. Client-side apply in `src/client/mod.rs`: maintain the last frame, apply
   deltas, request a keyframe on gap/resize.
4. Characterization test: a frame sequence differing by a few cells reconstructs
   bit-identically from keyframe + deltas. Gate: `just test-one wire`,
   `just test-one render` green.
5. Keep `SemanticFrame` (whole-frame) as the default; gate `FrameDelta` behind
   the existing `HERDR_RENDER_ENCODING` negotiation so it can be A/B tested.

### B. Make it default
6. After A is validated in real use, flip the default negotiation to the delta
   encoding. Gate: manual busy-pane + resize check shows no corruption; full
   suite green.

## STOP conditions
- If the retained dirty-patch bookkeeping does not actually capture every cell
  change the semantic frame reflects (e.g. graphics/hyperlink layers bypass it),
  STOP — a delta built from incomplete damage data will corrupt the client.
- If bumping `PROTOCOL_VERSION` would break the in-flight update/handoff compat
  matrix, coordinate with the maintainer before shipping.

done-when: proposal `frame-delta-encoding` archived after step B ships.
