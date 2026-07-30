# Frame-delta encoding for the default SemanticFrame client

Base commit: `1de05dc2` · Route: proposal · Effort: M-L · Confidence: HIGH · Category: performance

## Why

Resolves advisory finding PERF-01 (audit against `1de05dc2`). This is the single
largest transport-perf lever found.

The recent render-avoidance work ("avoid full pane renders for spinner
animation", "avoid renders for passive mouse motion") saves the *render* but not
the *transport*:

- `src/client/mod.rs:663-668` — the normal TUI client negotiates
  `RenderEncoding::SemanticFrame`; `TerminalAnsi` only when `HERDR_RENDER_ENCODING`
  is set.
- `src/server/render_stream.rs:65-112` (`prepare_frame`) — the `TerminalAnsi` arm
  runs a blit encoder and emits only changed bytes; the `Semantic` arm does an
  equality check then emits `ServerMessage::Frame(frame)` — the **entire**
  `FrameData`, no diff.
- `src/server/headless.rs:3454-3521` — even the retained/damage-tracked fast path
  patches 1-2 dirty rows and then serializes the whole frame to send it.

For a 200×50 terminal a bincode `FrameData` is ~70-150 KB. With `MIN_RENDER_INTERVAL`
at 16 ms (`src/app/mod.rs:35`), a busy pane pushes several MB/s over the socket,
plus a full-frame `PartialEq` of ~10,000 `CellData` (each with a `String`) per
send. The diffing machinery already exists — it just serves the secondary
`terminal attach` surface, not the default client.

Related allocation costs that this work should fold in: `CellData.symbol: String`
allocates per cell (`src/protocol/wire.rs:432-457`, PERF-02), and the retained
path deep-clones the previous frame before patching it
(`src/server/headless.rs:3454`, PERF-04).

## What changes

Add a `ServerMessage::FrameDelta { seq, base_seq, cell_runs }` wire message
alongside the existing `Frame` (kept as the keyframe/repaint path). Build the
delta from the dirty-row bookkeeping the retained path already computes, instead
of discarding it. Client applies the delta to its held frame. Bump
`PROTOCOL_VERSION` (`src/protocol/wire.rs:16`) and handle client/server version
compat.

Because this touches the wire protocol and is release-risk (protocol IDs, render
projection), treat it per AGENTS.md's refactor-risk process: name/add
characterization tests for the render-stream output before changing the encoder.

## Acceptance

- `just check` passes.
- `PROTOCOL_VERSION` bumped per the AGENTS.md rule (compare against latest
  released tag; bump only if not already ahead).
- A test feeds a sequence of frames differing by a few cells and asserts the
  client reconstructs each frame bit-identically from keyframe + deltas.
- A visual/manual check: a busy pane renders correctly with no corruption; a
  full repaint (resize) still works.
- Wire-format fixture tests updated.

## Out of scope / staging

- This is large; stage it: (1) land the `FrameDelta` message + client apply
  behind the existing encoding negotiation so it can be A/B'd via
  `HERDR_RENDER_ENCODING`; (2) make it the default; (3) fold in the `CellData`
  per-cell `String` reduction (PERF-02) and the retained-path clone removal
  (PERF-04) as follow-ups. Do not attempt all three in one change.
