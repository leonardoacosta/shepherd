# Design spike: a public pane-output subscription on the JSON API

Base commit: `1de05dc2` · Route: proposal (design spike) · Effort: L (coarse) · Confidence: HIGH that this is the stated direction's next increment · Category: direction

## Why

Resolves advisory finding DIRECTION-01 (audit against `1de05dc2`). This is a
maintainer decision, framed as a spike — define the API and open questions, don't
build it all.

AGENTS.md declares "Herdr is migrating toward a server-owned runtime protocol
with the TUI as one client." Today the TUI is **not** one client — it is the only
client that can see live pane content, because that content is delivered only as
pre-rendered cells over the private frame protocol (`src/protocol/wire.rs:16`,
`PROTOCOL_VERSION`; `src/server/render_stream.rs`; `src/protocol/render_ansi.rs`).
The public JSON API has `pane.read` (pull) and `pane.wait_for_output` (one-shot
matcher), and 27 event types of which `pane.output_matched` is the only
output-adjacent one — there is no continuous output event. README already sells
the socket API as "agents spawn panes, read output, wait on each other."

A public `pane.output` subscription is the smallest increment that makes "TUI as
one client" true, and it is what any third-party client (web UI, mobile attach, an
agent that wants to tail rather than poll `pane.read`) needs first. It directly
serves the sponsor pitch ("the path to a real agent runtime").

## What this spike produces

A written design (in `docs/next/` or a `.local/prd/` note, per AGENTS.md) plus a
minimal reference implementation or prototype, answering:

- **Event shape**: raw bytes vs. rendered text vs. both? What does a subscriber
  receive on subscribe (snapshot + tail, or tail-only)?
- **Backpressure**: an unbounded output stream over a unix socket is a
  memory-growth hazard on a slow consumer. Define a bounded ring + drop-with-gap-
  marker semantics from day one (the `render` capacity-1 slot in
  `src/server/client_transport.rs` is the existing precedent for "slow clients
  cannot build lag").
- **Compatibility surface**: once public, the output encoding is harder to change
  than the private protocol (which is version-gated and can break freely). Decide
  what is committed vs. experimental.
- **Fan-out**: reuse `src/api/subscriptions.rs` (already fans out events) vs. a
  dedicated path.

## Acceptance (spike)

- A design doc exists covering the four questions above with a recommendation.
- Either a prototype `pane.output` subscription behind an experimental flag, or a
  concrete API schema addition (`src/api/schema/`) with the backpressure policy
  specified — enough that a build proposal can be written from it.
- `just check` passes if any code lands.

## Explicitly not in scope

- Shipping a stable, committed `pane.output` API. This spike decides *whether* and
  *how*; the build is a follow-up proposal.
