# Tasks — memoize-agent-panel-entries

Base commit: `1de05dc2`. Drift check: confirm `agent_panel_entries` is defined in
`src/ui/sidebar.rs:~112` and called from `compute_view_internal`
(`src/ui.rs:~240`) via the scroll-metrics helpers and again during render
(`sidebar.rs:~846`). If the function moved or the call sites changed, STOP.

Exemplar: `ViewState` in `src/app/state.rs:775+` — how existing per-frame
computed geometry (e.g. `sidebar_rect`, hit areas) is stored on the view and
read by render. Add the entries field the same way.

## Ordered steps

1. **Add the cache field.**
   - Add `agent_panel_entries: Vec<AgentPanelEntry>` (and, if needed by hit-test
     callers, the derived scroll metrics) to `ViewState` in `src/app/state.rs`.
   - Gate: `cargo build` (via `just lint`) compiles.

2. **Populate it once.**
   - In `compute_view_internal` (`src/ui.rs`), compute the entries once and store
     them on the `ViewState` being built, before the scroll-metrics calls. Change
     `agent_panel_scroll_metrics` / `agent_panel_bottom_start` /
     `agent_panel_visible_count_from` to take the already-computed slice rather
     than rebuilding.

3. **Read the cache in render and hit-testing.**
   - `sidebar.rs:~846` render and `agent_panel_scrollbar_rect` (`:654`) read
     `app.view.agent_panel_entries` instead of recomputing.
   - Gate: `just test-one sidebar` and `just test-one agent_view` green (these
     assert entry ordering/content).

4. **Confirm no double compute remains on the frame path.**
   - Grep `agent_panel_entries(` and verify render/scroll/scrollbar paths all read
     the cache; only non-frame callers still call the function.

5. **Verify and close.**
   - `just check` passes. done-when: proposal `memoize-agent-panel-entries`
     archived.

## STOP conditions

- If any render-path caller needs entries computed against a *different*
  `AppState` than the one `compute_view_internal` saw (i.e. state mutates between
  compute and render), STOP — that would be a pre-existing violation of the
  "render is pure" rule and changes the premise.
