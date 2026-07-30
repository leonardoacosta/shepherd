# Build an experimental `pane.output` subscription on the JSON API

Base decision: `docs/next/pane-output-spike.md` (2026-07-30) · Route: proposal · Effort: M · Confidence: MED · Category: API

## Why

The spike concluded that the JSON API needs a continuous pane-output surface so
the TUI is not the only client with live pane content. `pane.read` and
`pane.wait_for_output` remain useful, but they are snapshot/poll primitives.

## What changes

1. Add an experimental `Subscription::PaneOutput` schema surface with:
   - `pane_id`
   - `source`
   - `format`
   - `initial_tail_lines`
2. Add `SubscriptionEventKind::PaneOutput` and payload fields:
   - `pane_id`, `source`, `format`, `revision`, `chunk`, `truncated`, `gap`, `initial`
3. Implement producer fan-out with per-subscriber bounded queues and gap markers.
4. Add an example consumer and drift-check coverage for the schema.

## Acceptance

- A subscriber can tail pane output end-to-end over the JSON API.
- Slow consumers do not cause unbounded memory growth.
- Dropped output is surfaced via gap markers, not silently lost.
- Schema/tests/docs stay in sync.

## Out of scope

- Stabilizing the API contract beyond the experimental shape.
- A raw PTY byte mode.
