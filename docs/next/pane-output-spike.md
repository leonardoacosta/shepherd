# Pane Output Subscription Spike

Date: 2026-07-30
Status: approved recommendation
Scope: design spike for a public `pane.output` subscription on the JSON API

## Current output paths

Today the public API exposes pane output in two pull-style forms:

- `pane.read` returns a snapshot from `src/app/api_helpers.rs::read_terminal_snapshot()`.
  It reads from `TerminalRuntime` snapshot methods:
  - `visible_text()` / `visible_ansi()`
  - `recent_text_snapshot()` / `recent_ansi_snapshot()`
  - `recent_unwrapped_text_snapshot()` / `recent_unwrapped_ansi_snapshot()`
  - `detection_text()` / `detection_ansi()`
- `pane.wait_for_output` in `src/api/wait.rs` polls `pane.read` every
  `CONNECTION_POLL_INTERVAL` until a matcher hits or times out.
- `events.subscribe` `pane.output_matched` in `src/api/subscriptions.rs`
  performs the same pattern: probe `pane.read`, then keep re-reading and emit an
  event when the matcher transitions into a match.

The TUI does not use the JSON API for live pane content. It receives private
frame-protocol payloads from `src/server/render_stream.rs` via
`src/protocol/wire.rs`. That means the public API still has no continuous output
surface, only repeated snapshots and one-shot matchers.

## Recommendation

Add a public `pane.output` subscription as an experimental JSON API surface in a
follow-up build proposal.

The design should commit to these answers:

### 1. Event shape

Use structured text/ANSI chunks, not raw PTY bytes.

Recommended subscription shape:

- subscription selector:
  - `pane_id`
  - `source`: `recent`, `recent_unwrapped`, or `visible`
  - `format`: `text` or `ansi`
  - `initial_tail_lines`: optional bootstrap snapshot size
- streamed event payload:
  - `pane_id`
  - `source`
  - `format`
  - `revision`
  - `chunk`
  - `truncated`: whether the producer had to trim the chunk itself
  - `gap`: optional marker describing dropped content
  - `initial`: `true` only for the bootstrap tail event sent immediately after
    subscribe

Reasons:

- Raw PTY bytes would freeze a lower-level compatibility contract than the
  current API exposes.
- `pane.read` already defines the user-meaningful output surfaces, so the stream
  should speak the same language.
- `ansi` remains available for consumers that need styling fidelity.

### 2. Backpressure

Per-subscriber bounded ring buffer with explicit gap markers.

Recommended policy:

- one bounded queue per subscriber, capped by both message count and bytes
- newest data is retained; oldest droppable output chunks are evicted first
- when eviction occurs, enqueue a synthetic gap marker before the next real chunk
- if even the gap marker cannot be queued, disconnect the subscriber

Reasons:

- This matches the existing server precedent that slow consumers must not create
  unbounded memory growth.
- A gap marker is strictly better than silent loss for logs, web UIs, and tailing
  agents.
- Disconnecting only after the buffer cannot even represent loss keeps normal
  slow consumers usable.

### 3. Compatibility surface

Mark `pane.output` experimental in its first public version.

Commit only to:

- the subscription name
- the top-level fields in the event payload
- bounded delivery with possible gap markers

Do not commit yet to:

- exact chunk segmentation boundaries
- whether future versions also offer a raw-byte mode
- replay semantics beyond the initial tail event

Reasons:

- Chunking strategy is likely to evolve as the runtime/server boundary changes.
- The first implementation should preserve room to switch from polling-driven
  chunks to runtime-native append buffers without a public break.

### 4. Fan-out

Reuse `src/api/subscriptions.rs` and `EventHub` conventions for subscription
setup and event framing, but use a dedicated output producer path rather than
faking live output as a normal `EventKind`.

Reasons:

- Subscription validation, request IDs, and response framing already exist there.
- Live pane output has materially different backpressure needs from ordinary
  lifecycle events.
- A dedicated producer path avoids teaching the global event bus to carry large
  continuous payloads.

## Proposed build sequence

1. Add schema support for `Subscription::PaneOutput` and
   `SubscriptionEventKind::PaneOutput`.
2. Keep it undocumented or explicitly experimental on first landing.
3. Implement output fan-out with bounded per-subscriber queues and gap markers.
4. Add a smoke consumer example.
5. After real usage, decide whether to stabilize the shape unchanged or add a raw
   output mode separately.

## Determination

Proceed with a follow-up build proposal. Do not keep the private frame protocol
as the only live pane-output surface.
