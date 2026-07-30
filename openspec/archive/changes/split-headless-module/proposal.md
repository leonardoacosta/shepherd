# Split the server/headless.rs god module and neutralize toast-driven notifications

Base commit: `1de05dc2` · Route: proposal · Effort: L (incremental M chunks) · Confidence: HIGH · Category: architecture

## Why

Resolves advisory findings DEBT-02 and DEBT-03 (audit against `1de05dc2`).
AGENTS.md: "No god objects. If a module is doing too many things, split it."

- `src/server/headless.rs` is 9,859 lines (test module starts ~4,472, so ~4,472
  production lines) with 234 function definitions in essentially one `impl`, and
  is the #2 churn file over the last 100 commits. Its module doc lists ten
  responsibilities: main loop (`:481-720`), client lifecycle/foreground
  arbitration (`:930-1042`), live handoff (`:1044+`), notification forwarding
  (`:1996-2300`), API request draining (`:2983-3200`), retained rendering
  (`:3400-3535`), full rendering + streaming (`:3612-3860`), scheduled tasks
  (`:3895-3990`). The split pattern already exists — `src/server/` has
  `client_accept.rs`, `client_transport.rs`, `clients.rs`, `keybindings.rs`,
  `notifications.rs`, `socket_paths.rs`, `terminal_attach.rs`, `render_stream.rs`
  — `headless.rs` just never shrank into it.
- Notification routing is driven by diffing a **TUI presentation field**:
  `src/server/headless.rs:2025,2081,2119,2173,...` repeat
  `let toast_before = self.app.state.toast.clone(); ...; if toast changed {
  forward }` at eight-plus sites, with `.expect("toast forwarding requires a
  client notification kind")` in production code (`:1820,2105,2197,...`) and the
  server owning the toast dismissal timer (`:3913`). AGENTS.md's runtime/client
  guardrail names toasts as TUI/client presentation state and says to use neutral
  server/API names. Today the server cannot tell a client "a runtime event
  happened" without first rendering it into a TUI toast and diffing the result.

## What changes

Two coordinated tracks, executed incrementally (each chunk is behavior-preserving
code motion, validated before the next):

1. **Extract cohesive clusters** from `headless.rs` into existing/new sibling
   modules: retained + full render/stream → a `server/render_loop.rs`;
   notification forwarding → the existing `server/notifications.rs`; live handoff
   → the existing `server/handoff.rs`; scheduled tasks → their own module. Target
   `headless.rs` holding only the main loop + client lifecycle.
2. **Neutralize notifications**: introduce a neutral
   `RuntimeNotification { kind, title, body, source }` emitted by the handlers
   themselves onto an event channel. The server forwards from that channel; the
   TUI client renders it as a toast and owns the dismissal timer. Delete the
   before/after `toast` clone sites and the `expect`s.

This is release-risk (touches a core surface and UI/input state projection) —
follow AGENTS.md: identify protected behavior and add/name characterization tests
before moving code; use `AppState::assert_invariants_for_test()` where identity
state is touched.

## Acceptance

- `just check` passes after each chunk.
- Notification delivery is unchanged: the `herdr`-in-frame vs client-local
  delivery-mode branching is preserved and covered by a test.
- No `.expect(...)` remains in the notification path in production code.
- `headless.rs` production line count is materially reduced (target: main loop +
  client lifecycle only).

## Out of scope

- Behavior changes of any kind during the extraction chunks — pure motion.
- The larger `AppState` view/runtime split (DEBT-01) — that is a separate,
  bigger program; this change only relocates notifications off the `toast` field,
  not the whole view-state separation.
