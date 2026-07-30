# Tasks — render-golden-corpus

Base commit: `1de05dc2`. Drift check: confirm `src/server/render_stream.rs:65-112`
still has the `TerminalAnsi`/`Semantic` arms and `src/protocol/wire.rs` still
defines `FrameData`/`CellData` with the length-prefixed framing. If the encoder was
already rewritten, coordinate with `frame-delta-encoding`.

Exemplar: the `detection-golden-corpus` proposal (same capture→assert shape) and
existing `src/protocol/wire.rs` fixture tests for the frame serialization style.

## Ordered steps

1. **Pick a deterministic render entry point.** Find the smallest function that
   turns a pane's processed byte stream into a `FrameData` (via the virtual
   ratatui buffer). Confirm it's deterministic given fixed input + size.

2. **Author fixtures.** Under `tests/fixtures/render/`, store input byte streams +
   expected frames for: plain text, cursor movement, wide grapheme/emoji, scroll-
   region clear, and a resize. Normalize any nondeterministic fields.

3. **Whole-frame golden test.** Feed each input, render, assert the frame equals
   the golden. Land this now — it's the safety net.
   - Gate: `just test-one render` green.

4. **Delta-equivalence test (enable with FrameDelta).** Add a test that replays the
   sequence through keyframe+delta and asserts byte-identical reconstruction vs the
   whole-frame path. Guard it `#[ignore]` / feature-gated until `FrameDelta` exists,
   then enable it inside the `frame-delta-encoding` change.
   - Gate: once enabled, `just test-one wire` + `just test-one render` green.

5. **Verify and close.**
   - `just check` passes. done-when: proposal `render-golden-corpus` archived (with
     the delta-equivalence assertion handed to `frame-delta-encoding` to enable).

## STOP conditions

- If capturing a stable expected `FrameData` requires freezing state the renderer
  legitimately varies (e.g. a blink phase), STOP and normalize it out of the
  fixture rather than pinning a flaky value.
