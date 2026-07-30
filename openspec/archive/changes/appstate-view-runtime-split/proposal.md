# Extract per-client view state from AppState (runtime/client boundary)

Base commit: `1de05dc2` · Route: proposal (multi-release program) · Effort: L · Confidence: HIGH on the problem; the sequencing is the risk · Category: architecture

## Why

Adjacent north-star follow-on resolving advisory finding DEBT-01 — the change the
runtime/client-boundary work (`split-headless-module` notifications, and
`spike-api-pane-output-stream`) has been pointing at. AGENTS.md states the
explicit direction: "Herdr is migrating toward a server-owned runtime protocol
with the TUI as one client. New work should not deepen the current server/TUI
coupling."

`AppState` today is both the server session model and one TUI's view model:

- `src/app/state.rs:~1417` — `AppState` has ~116 fields, including
  `sidebar_width`, `sidebar_collapsed`, `sidebar_section_split`, `workspace_scroll`,
  `agent_panel_scroll`, `tab_scroll`, `mobile_switcher_scroll`, `mouse_capture`,
  `copy_on_select`, `toast`, `context_menu`, `palette`, `theme_name`, `global_menu`,
  and `view: ViewState`.
- `src/app/state.rs:~775-790` — `ViewState` stores pixel-level TUI hit-testing
  geometry on the server: `sidebar_rect`, `workspace_card_areas`, `tab_hit_areas`,
  `new_tab_hit_area`, `toast_hit_area`, `split_borders`.
- `src/app/state.rs:~792-814` — `Mode` enumerates ~20 TUI screens (`Settings`,
  `KeybindHelp`, `ContextMenu`, `RenameWorkspace`, `ReleaseNotes`, `Navigator`, …).
- `src/server/headless.rs:~930-988` — `resize_shared_runtime_to_effective_size`
  pins all pane runtimes to the *foreground TUI client's* geometry, which is why
  `compute_view_without_resizing_panes` (`src/ui.rs:~144`) has to exist.

AGENTS.md's guardrail assigns "Sidebar layout, token placement, colors, selection,
modals, mouse/viewport state" to the TUI/client layer, and only settles
"Workspace/tab/pane remain shared session organization for now" — sidebar geometry
and modal `Mode` are **not** covered by that settled tradeoff. Consequences: every
non-TUI client inherits a model shaped by one TUI's layout; multi-client is
structurally limited to one "foreground" client; and several perf findings
(memoization, per-frame buffer churn) are downstream of view geometry living in
server state.

## What changes (staged — a multi-release program, not one change)

Do **not** split `AppState` in one go. Draw the line at the newest surface first:

1. **Freeze `ViewState`** — stop adding fields to it; add a lint/CI note.
2. Extract `ViewState` + `sidebar_*` / `*_scroll` / `mouse_*` / `Mode` into a
   `ClientViewState` owned per-client, with the server exposing only runtime facts.
3. Re-express the pieces that legitimately need to be shared (per AGENTS.md's
   "shared runtime/session fact") through the JSON API/event path, using neutral
   names — not UI-surface names.

This is release-risk by the repo's own classification (persisted state,
restore/handoff, workspace/tab/pane identity, UI/input state projection). Follow
AGENTS.md: name/add characterization tests before moving code; use
`AppState::assert_invariants_for_test()` and
`AppState::test_with_adversarial_identity_state()`; run a roundtable.

## Acceptance (per stage — each ships independently)

- `just check` passes at every stage.
- `ViewState` gains no new fields after the freeze (a check enforces it).
- After extraction, a headless/non-TUI path can operate without constructing TUI
  view geometry (demonstrated by a test that exercises the server with no
  foreground client view).
- Restore/handoff round-trips are covered by characterization tests written
  *before* the move and still green after.
- No regression in the TUI (frame snapshots / existing UI tests green).

## Out of scope

- Doing all three stages at once.
- The settled workspace/tab/pane-as-shared-organization tradeoff — leave it.

## Dependencies

- Best sequenced **after** `split-headless-module` (notifications off the `toast`
  field) and informed by `spike-api-pane-output-stream` (what "runtime fact" the
  API should expose). This is the terminus, not the starting point.

## STOP conditions

- If extracting any field breaks a persisted-state or protocol-ID contract,
  STOP — that needs its own migration proposal and a protocol-version decision,
  not an inline change.
- If a "TUI-only" field turns out to be read by the server's runtime logic
  (genuine coupling), capture that dependency explicitly before moving it — it may
  be a real shared fact that belongs in the API, not the client.
