This change depends on `anchor-chrome-to-sidebar`. That proposal rewrites the root layout split
in `src/ui.rs` and the sidebar render path in `src/ui/sidebar/`. Do not start section 3 until it
has landed; sections 1 and 2 are independent of it.

## 1. Characterize the behaviour being replaced

- [ ] 1.1 Before moving any code, write characterization tests over the current two-section
      sidebar covering: agent-row activation of a non-active space and a non-active tab,
      shortcut numbering by visible position, worktree-child indentation and chevron collapse,
      space drag-reorder, and scrolled-row hit-testing. These must pass against the current
      implementation first.
- [ ] 1.2 Add a restore test loading a session snapshot that carries `sidebar_section_split`,
      asserting the current code restores it, so the post-removal test in 2.5 can assert the
      value is ignored rather than rejected.
- [ ] 1.3 Run `cargo nextest run sidebar` and paste the passing output as the pre-change
      baseline.

## 2. Remove the section split

- [ ] 2.1 Remove `sidebar_section_split` from `AppState` and `ClientViewState` in
      `src/app/state.rs`, including the field-name test list.
- [ ] 2.2 Remove the divider drag target and its hit-testing from `src/app/input/sidebar.rs`,
      `src/app/input/mouse.rs`, `src/app/input/mod.rs`, and `src/app/input/terminal.rs`.
- [ ] 2.3 Remove the persisted field from `src/persist/snapshot.rs`, `src/persist/restore.rs`,
      and `src/persist/io.rs`. Do not add a migration — `SessionSnapshot` carries no
      `deny_unknown_fields`, so an older snapshot's key is ignored.
- [ ] 2.4 Remove the handoff mirrors from `src/server/headless.rs`,
      `src/server/headless/live_handoff.rs`, and `src/server/handoff.rs`, and the seed/restore
      paths in `src/app/mod.rs` and `src/app/actions.rs`.
- [ ] 2.5 Convert the 1.2 baseline test to assert an older snapshot restores without error and
      the value is ignored.
- [ ] 2.6 Confirm no protocol bump is required: `src/protocol/wire.rs::PROTOCOL_VERSION` is 19
      and the latest released tag carries 17, so the source protocol already exceeds the released
      protocol. Paste both values. Do not bump.
- [ ] 2.7 Run `cargo nextest run` and paste the passing output.

## 3. Build the merged row model

- [ ] 3.1 In `src/ui/sidebar.rs`, add one row builder producing the merged list: a space row
      followed by that space's agent rows, then the next space. Emit no tab rows. Reuse
      `agent_panel_entries_from` for the agent rows rather than re-deriving pane state.
- [ ] 3.2 Apply the persisted agent sort within each space's agent rows, reading the same
      `ui.agent_panel_sort` source the current panel reads. Do not add a second state source.
- [ ] 3.3 Emit exactly one empty-state row for a space with no detected agent, marked
      non-selectable.
- [ ] 3.4 Keep worktree children indented beneath their parent space with their existing tree
      glyphs, and keep the parent's collapse chevron collapsing both the children and their
      agents.
- [ ] 3.5 Add row-model tests: agent rows sit under their own space in space order; no tab rows
      are emitted for a multi-tab space; one empty-state row per agentless space; a collapsed
      space emits no child rows.
- [ ] 3.6 Run `cargo nextest run sidebar` and paste the passing output.

## 4. Render and interact with the merged list

- [ ] 4.1 Replace the two-section render in `src/ui/sidebar.rs` with a single list render.
      Delete `expanded_sidebar_sections`, `workspace_list_rect`, and the divider render once no
      caller remains; verify zero remaining callers with a repo-wide search excluding
      `openspec/changes/archive/`.
- [ ] 4.2 Rework `workspace_card_areas` in `src/app/state.rs` into merged row areas covering both
      space and agent rows, and update every hit-test consumer.
- [ ] 4.3 Unify the two scroll offsets into one scroll model for the merged list, and update the
      scrollbar metrics.
- [ ] 4.4 Assign shortcut numbers by visible position across the whole merged list.
- [ ] 4.5 Update `src/ui.rs` for the single-region sidebar area.
- [ ] 4.6 Port the 1.1 characterization tests to the merged list; they must pass unchanged in
      intent — activation, numbering, grouping, reorder, and scrolled hit-testing.
- [ ] 4.7 Run `cargo nextest run` and paste the passing output.

## 5. Keep the other sidebar presentations working

- [ ] 5.1 Update the collapsed sidebar in `src/ui/sidebar.rs` to the flat model, keeping its
      compact space-then-agent ordering and its existing click targets.
- [ ] 5.2 Verify `src/ui/mobile.rs` consumers — `agent_panel_entries`,
      `agent_panel_entries_from`, `agent_panel_status_key` — still resolve, and update them if
      the merged builder changes their shape.
- [ ] 5.3 Add tests covering the collapsed presentation's ordering and the mobile consumers.
- [ ] 5.4 Run `cargo nextest run mobile` and `cargo nextest run sidebar`, and paste both passing
      outputs.

## 6. Correct the spec record and document

- [ ] 6.1 Apply the `space-agent-navigation` requirements.
- [ ] 6.2 Apply the modified `everyday-ui-preferences` agent-sort requirement.
- [ ] 6.3 Document the merged sidebar and the removed section-split configuration in
      `docs/next/website/src/content/docs/`, and add a `docs/next/CHANGELOG.md` entry recording
      the removal as a breaking configuration change.
- [ ] 6.4 Run `just check` and paste the passing output.
- [ ] 6.5 Manually verify against a live server: open three spaces, one running two agents across
      two tabs, one running a single agent, and one running none. Confirm the flat ordering, the
      empty-state row, activation from an inactive space, and worktree-child collapse. Paste the
      observed rows.
