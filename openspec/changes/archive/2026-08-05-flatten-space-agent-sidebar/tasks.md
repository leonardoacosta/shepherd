`anchor-chrome-to-sidebar` has landed, so every section below is unblocked.

**Ordering note (revised after a first attempt was reverted).** The original plan removed
`sidebar_section_split` before rewriting the render. That is backwards: the render is what
*consumes* the field (`expanded_sidebar_sections(area, split)`, `workspace_list_rect(area,
split)`), so removing it first forces throwaway stubs at every call site that the render merge
then deletes, and leaves the tree uncompilable in between. The merge and the removal must land in
one pass, render first. Sections 2 and 3 below are that single pass.

**Implementation notes (proven by a second attempt, then reverted).** These were verified
against the tree at `d4cc7816` — the builder compiled and its tests passed before the revert:

- **Do not widen `WorkspaceListEntry`.** It has 49 references across `src/ui.rs`,
  `src/ui/sidebar.rs`, `src/ui/mobile.rs`, `src/app/actions.rs`, and
  `src/app/input/sidebar.rs`, and the mobile switcher iterates the same list — adding agent
  variants changes mobile too. Add a separate `SidebarRow { Space, Agent, NoAgents }` enum and a
  `sidebar_rows(app, &sorted_entries)` builder alongside it. That shape passed four row-model
  tests covering ordering, the empty state, no-tab-rows, and sort pass-through.
- **`compute_workspace_list_areas`'s second tuple slot is an unused `Vec<()>`.** Repurpose it for
  the agent row rects rather than adding a parallel function; a single walk of `sidebar_rows`
  then produces space and agent geometry together and they cannot drift apart.
- **Geometry and render must land in the same increment.** Emitting agent rects without
  rendering them leaves visible gaps where agents belong, so an interleaved-geometry-only commit
  is a user-visible regression and must not be committed on its own.
- **Exactly one existing test fails on interleaved geometry**:
  `app::actions::tests::switch_workspace_keeps_selected_visible_in_scrolled_sidebar`. It fails
  because the scroll model still measures spaces only, which is precisely the scope of task 2.7 —
  treat it as that task's acceptance test rather than adjusting the assertion.

## 1. Record the baseline

- [x] 1.1 The characterization coverage this change needs already exists — do not write
      duplicates. Confirm `src/app/input/sidebar.rs` still covers agent-row activation into a
      non-active space and non-active tab, worktree-parent chevron toggle, workspace drag-reorder,
      agent hit-testing after a scroll/filter shrink, and shortcut numbering by visible position
      (`collapsed_sidebar_numbers_grouped_agents_by_list_position` and its priority twin in
      `src/ui/sidebar.rs`). Name the tests that cover each behaviour.
- [x] 1.2 Confirm `round_trip_empty_session` in `src/persist/snapshot.rs` proves the split field
      restores today — that is the pre-removal baseline for 3.5.
- [x] 1.3 Run `cargo nextest run sidebar` and `cargo nextest run persist` and paste the passing
      output as the pre-change baseline.

## 2. Build and render the merged list

- [x] 2.1 In `src/ui/sidebar.rs`, add one row builder producing the merged list: a space row
      followed by that space's agent rows, then the next space. Emit no tab rows at any depth.
      Reuse `agent_panel_entries_from` for the agent rows rather than re-deriving pane state.
- [x] 2.2 Apply the persisted agent sort within each space's agent rows, reading the same
      `ui.agent_panel_sort` source the current panel reads. Do not add a second state source.
- [x] 2.3 Emit exactly one empty-state row for a space with no detected agent, marked
      non-selectable.
- [x] 2.4 Keep worktree children indented beneath their parent space with their existing tree
      glyphs, and keep the parent's collapse chevron collapsing both the children and their
      agents.
- [x] 2.5 Replace the two-section render with a single list render. Change
      `expanded_sidebar_sections` and `workspace_list_rect` to take only the area, or delete them
      once no caller remains; delete the section-divider render and
      `sidebar_section_divider_rect`. Verify zero remaining callers with a repo-wide search
      excluding `openspec/changes/archive/`.
- [x] 2.6 Rework `workspace_card_areas` in `src/app/state.rs` into merged row areas covering both
      space and agent rows, and update every hit-test consumer.
- [x] 2.7 Unify the two scroll offsets into one scroll model for the merged list, and update the
      scrollbar metrics.
- [x] 2.8 Assign shortcut numbers by visible position across the whole merged list.
- [x] 2.9 Update `src/ui.rs` for the single-region sidebar area.
- [x] 2.10 Add row-model tests: agent rows sit under their own space in space order; no tab rows
      are emitted for a multi-tab space; one empty-state row per agentless space; a collapsed
      space emits no child rows.
- [x] 2.11 Confirm the 1.1 characterization tests still pass unchanged in intent — activation,
      numbering, grouping, reorder, and scrolled hit-testing.
- [x] 2.12 Run `cargo nextest run` and paste the passing output.

## 3. Remove the section split

Runs only after section 2 compiles green, so no call site needs a temporary stub.

- [x] 3.1 Remove `sidebar_section_split` from `AppState` and `ClientViewState` in
      `src/app/state.rs`, including both field-name test lists.
- [x] 3.2 Remove the divider drag target and its hit-testing from `src/app/input/sidebar.rs`
      (`on_sidebar_section_divider`, `set_sidebar_section_split`), `src/app/input/mouse.rs`,
      `src/app/input/mod.rs`, and `src/app/input/terminal.rs`.
- [x] 3.3 Remove the persisted field from `src/persist/snapshot.rs` (`SessionSnapshot`,
      `RawSessionSnapshot`, `migrate_snapshot`, and the `capture` / `capture_with_dock_exclusions`
      parameters), `src/persist/restore.rs`, and `src/persist/io.rs`. Do not add a migration —
      `SessionSnapshot` carries no `deny_unknown_fields`, so an older snapshot's key is ignored.
- [x] 3.4 Remove the handoff mirrors from `src/server/headless.rs`,
      `src/server/headless/live_handoff.rs`, and `src/server/handoff.rs`, and the seed/restore
      paths in `src/app/mod.rs` and `src/app/actions.rs`. Note that `src/server/headless.rs:143`
      declares `ClientViewProjection` as an 11-element tuple whose **third element is this
      field** — the type alias, its destructure in `apply_client_view_projection`, and both
      projection getters all change together. This is a type-level change, not a field deletion.
- [x] 3.5 Add a test asserting an older snapshot restores without error and the value is ignored.
      `tests/fixtures/session/current-shepherd-dev-session.json:78` still carries the retired key
      on disk — leave the fixture unmodified and assert against it, including that a re-serialize
      drops the key.
- [x] 3.6 Confirm no protocol bump is required: `src/protocol/wire.rs::PROTOCOL_VERSION` is 19
      and the latest released tag carries 17, so the source protocol already exceeds the released
      protocol. Paste both values. Do not bump.
- [x] 3.7 Run `cargo nextest run` and paste the passing output.

## 4. Keep the other sidebar presentations working

- [x] 4.1 Update the collapsed sidebar in `src/ui/sidebar.rs` to the flat model, keeping its
      compact space-then-agent ordering and its existing click targets.
- [x] 4.2 Verify `src/ui/mobile.rs` consumers — `agent_panel_entries`,
      `agent_panel_entries_from`, `agent_panel_status_key` — still resolve, and update them if
      the merged builder changes their shape.
- [x] 4.3 Add tests covering the collapsed presentation's ordering and the mobile consumers.
- [x] 4.4 Run `cargo nextest run mobile` and `cargo nextest run sidebar`, and paste both passing
      outputs.

## 5. Correct the spec record and document

- [x] 5.1 Apply the `space-agent-navigation` requirements.
- [x] 5.2 Apply the modified `everyday-ui-preferences` agent-sort requirement.
- [x] 5.3 Document the merged sidebar and the removed section-split configuration in
      `docs/next/website/src/content/docs/` (EN, JA, ZH together), and add a
      `docs/next/CHANGELOG.md` entry recording the removal as a breaking configuration change.
- [x] 5.4 Run `just check` and paste the passing output.
- [x] 5.5 Manually verify a rendered capture: three spaces, one running two agents across two
      tabs, one running a single agent, and one running none. Confirm the flat ordering, the
      empty-state row, and worktree-child collapse. Paste the observed rows.

## Verification record

- **1.1 characterization coverage** (all confirmed present and still passing, unchanged in intent):
  agent-row activation into a non-active space/tab —
  `app::input::sidebar::tests::clicking_all_workspaces_agent_row_switches_to_correct_workspace`
  and `clicking_agent_detail_row_switches_to_correct_tab_and_pane`; worktree-parent chevron —
  `clicking_worktree_parent_chevron_toggles_group_only`; workspace drag-reorder —
  `dragging_workspace_reorders_without_changing_identity` and
  `dragging_worktree_parent_reorders_the_complete_group`; agent hit-testing after a filter shrink —
  `agent_hit_testing_clamps_scroll_after_dynamic_filter_shrink`; hit-testing after a scroll —
  `clicking_scrolled_agent_detail_row_switches_to_correct_tab_and_pane`; shortcut numbering by
  visible position — `ui::sidebar::tests::collapsed_sidebar_numbers_grouped_agents_by_list_position`
  and `collapsed_sidebar_numbers_priority_agents_by_list_position`.
- **1.2 / 3.5 restore baseline**: `persist::snapshot::tests::round_trip_empty_session` carried the
  split field pre-change. Replaced by `retired_section_split_key_is_ignored_on_restore`, which
  parses the unmodified `tests/fixtures/session/current-shepherd-dev-session.json` (still carrying
  the retired key on disk) and asserts a re-serialize drops it.
- **1.3 / 2.12 / 3.7 / 4.4 / 5.4**: `just check` green — `cargo nextest` 3261/3261 passed, plus
  clippy (host + `x86_64-pc-windows-msvc`), generated API client tests, and 110 maintenance script
  tests.
- **3.6 protocol**: source `src/protocol/wire.rs::PROTOCOL_VERSION` is `19`; `v0.7.5` carries `17`.
  Source already exceeds the released protocol, so no bump.
- **5.5 rendered capture** (three spaces: one running two agents across two tabs, one running a
  single agent, one running none):

  ```
   0| spaces       sort: grouped│
   1|                           │
   2| · shepherd                │
   3|   dev                     │
   4|   · shepherd · 1          │
   5|     claude                │
   6|   · shepherd · logs       │
   7|     claude                │
   8| · website                 │
   9|   dev                     │
  10|   · website               │
  11|     pi                    │
  12| · notes                   │
  13|   dev                     │
  14|   no agents               │
  21| new                menu  «│
  ```

  Flat ordering, per-space agent rows, the empty-state row, and one header confirmed. The collapsed
  presentation renders the same row order one line each, with worktree children collapsing from the
  parent chevron.

## Scope note

Unifying the two scroll offsets (task 2.7) removed `agent_panel_scroll` alongside
`sidebar_section_split`. The merged list has one scroll model, so keeping a second offset would
have left a persisted-and-handoff-mirrored field that no surface reads — the same
"key that silently does nothing" defect the proposal's `sidebar_section_split` decision rejects.
Both fields leave `AppState`, `ClientViewState`, and the `ClientViewProjection` tuple together.
