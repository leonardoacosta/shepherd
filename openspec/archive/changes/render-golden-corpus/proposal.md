# Build a render/frame-delta regression corpus

Base commit: `1de05dc2` · Route: proposal · Effort: M · Confidence: HIGH · Category: test coverage

## Why

Adjacent follow-on that **de-risks `frame-delta-encoding` and
`split-headless-module`** — the two corruption-prone changes in the batch. It
applies the "capture real state → assert" pattern established by
`detection-golden-corpus` to the render path.

The render/transport path is where a delta encoder or a render-loop extraction can
silently corrupt output:

- `src/server/render_stream.rs:65-112` (`prepare_frame`) — the encoder arms
  (`TerminalAnsi` blit vs `Semantic` whole-frame) that `frame-delta-encoding` will
  add a `FrameDelta` arm to.
- `src/protocol/wire.rs` — `FrameData` / `CellData` and the framing
  (length-prefixed, `MAX_FRAME_SIZE`); existing fixture tests here pin the wire
  format but not full rendered-screen reconstruction across a diff sequence.
- `src/protocol/render_ansi.rs` — the cell/row diffing the delta work reuses.

Today there is no golden corpus asserting "given this input byte stream, the
client reconstructs exactly this screen" — so a delta bug or a lost dirty-row in
the extracted render loop would pass CI.

## What changes

Commit a small corpus of input→expected-frame cases under
`tests/fixtures/render/` (a byte stream fed to a pane + the expected final
`FrameData`/rendered cells), plus a test that: (a) renders the input and asserts
the full frame matches the golden, and (b) once `FrameDelta` exists, replays the
sequence through keyframe+delta encoding and asserts the reconstruction is
bit-identical to the whole-frame path. Include the regression-prone cases: wide
graphemes/emoji, cursor movement, scroll region clears, and a resize.

## Acceptance

- `just check` passes.
- `tests/fixtures/render/` holds ≥5 input→frame cases including at least one
  wide-grapheme and one resize case.
- A test asserts whole-frame rendering matches each golden.
- A test asserts (guarded/skipped until `FrameDelta` lands, then enabled)
  keyframe+delta reconstruction equals the whole-frame result byte-for-byte.
- `just test-one render` / `just test-one wire` green.

## Out of scope

- Changing the encoder — this is coverage that the delta work depends on, not the
  delta work itself.
- Perf benchmarking — that's `render_prof`; this is correctness pinning.

## Dependencies

- Land the whole-frame golden part **before** `frame-delta-encoding` so the delta
  work has a safety net; enable the delta-equivalence assertion as part of that
  change.

## STOP conditions

- If rendered frames are not deterministic across runs (timestamps, nondeterministic
  ordering in `FrameData`), STOP and report — the corpus needs a deterministic
  normalization first, or the goldens will be flaky.
