# Tasks — split-headless-module

Base commit: `1de05dc2`. Drift check: confirm `src/server/headless.rs` is still a
single large `impl` (~4,472 production lines) and the toast-diff pattern
(`let toast_before = self.app.state.toast.clone();`) still appears at multiple
sites. If it has already been refactored, STOP and report scope.

Exemplar: existing extracted modules `src/server/notifications.rs`,
`src/server/render_stream.rs`, `src/server/terminal_attach.rs` — the module
boundary style to imitate. For characterization/invariants:
`AppState::assert_invariants_for_test()` and
`AppState::test_with_adversarial_identity_state()` per AGENTS.md.

## Ordered steps (each chunk lands and passes `just check` before the next)

1. **Name protected behavior + add characterization tests.**
   - Before moving anything, list the observable contracts: notification delivery
     modes, foreground-client arbitration, retained-vs-full render selection.
     Add/point to tests pinning each. Gate: those tests green on the unchanged
     module.

2. **Extract render loop.** Move retained + full render/stream methods into
   `server/render_loop.rs`, passing explicit params rather than relying on the
   shared `&mut self`. Gate: `just test-one render` + `just check` green.

3. **Extract scheduled tasks** and **live handoff** into their modules
   (`handoff.rs` already exists). Gate: `just test-one handoff` + `just check`.

4. **Introduce `RuntimeNotification` and move forwarding.**
   - Define the neutral struct; have API/event handlers emit it directly onto a
     channel. Server forwards from the channel into `server/notifications.rs`.
   - Move the toast dismissal timer to the TUI client; delete the before/after
     `toast` clone sites and the `.expect(...)` calls.
   - Gate: notification-delivery test green (both delivery modes); no `.expect`
     in the notification path (`grep -n 'expect(' src/server/notifications.rs`
     returns nothing in production code).

5. **Verify and close.**
   - `just check` passes. done-when: proposal `split-headless-module` archived.

## STOP conditions

- If moving a cluster requires changing a public API/protocol ID or persisted
  state shape, STOP — that is beyond behavior-preserving motion and needs its own
  proposal.
- If a notification currently depends on reading `toast` set by *another*
  handler (cross-handler coupling through the field), capture that dependency
  before deleting the field — it must be re-expressed as an explicit
  `RuntimeNotification`, not dropped.
