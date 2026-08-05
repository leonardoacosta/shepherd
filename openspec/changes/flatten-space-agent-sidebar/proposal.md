## Why

The sidebar renders the same tree twice, in two stacked sections, and neither section is the
whole truth.

The upper section lists spaces. The lower section lists agent panes. They are split by
`sidebar_section_split`, a draggable ratio. The lower section's default sort is
`AgentPanelSort::Spaces`, whose label is literally `grouped` — it is already ordered by the
space each agent belongs to. So the two sections agree on the hierarchy and disagree only on
whether to draw it: the upper one shows parents without children, the lower one shows children
whose parent is implied by ordering alone.

An operator reading the lower section cannot see which space a row belongs to without counting
rows against the upper section. An operator reading the upper section cannot see what is running
inside a space without looking down. Two panels, one relationship, expressed in neither.

The organising unit the operator actually works in is the project. A space already is one: it
carries `identity_cwd`, a discovered `GitSpaceMetadata`, a branch, and ahead/behind counts. What
runs inside it is a set of agents. Those agents are what the operator switches between, which is
what a tab is for.

So the sidebar becomes one flat list: a space row, then its agents directly beneath it, and
nothing else. No second section, no tab rows, no divider.

## What Changes

- Replace the two sidebar sections with one flat list: space rows, each followed by its own
  agent rows.
- Drop tab rows from the sidebar entirely. An agent row is the switch target; selecting one
  activates its tab and focuses its pane, as the agent panel already does today.
- Remove `sidebar_section_split` and the section divider — the config field, the drag handling,
  the `ClientViewState` field, the persisted snapshot field, and the handoff mirrors.
- Render a space that is running no detected agent with an explicit empty-state row rather than
  a bare space row, so "nothing running here" is visible rather than inferred from absence.
- Preserve every behaviour the two sections carry today: worktree grouping and its collapse
  chevron, agent sort as a single persisted source, per-row shortcut numbering by visible
  position, drag-reorder of spaces, scrollbars, and the collapsed and mobile presentations.

The runtime is not touched. Workspaces, tabs, panes, and their identities are unchanged, and no
API or event gains or loses a field. This is a client-side projection change, which is where
CLAUDE.md places sidebar layout and selection state.

## Capabilities

### New Capabilities

- `space-agent-navigation` — the sidebar's flat space-and-agent projection, its empty state, and
  the behaviours preserved across the merge.

### Modified Capabilities

- `everyday-ui-preferences` — the agent sort preference survives, but its requirement describes a
  panel that no longer exists as a separate section. The requirement is restated against the
  merged list without changing the one-persisted-source guarantee.

## Impact

- `src/ui/sidebar.rs` — the merge itself: one row builder, one scroll model, one hit-test model,
  replacing `expanded_sidebar_sections`, `workspace_list_rect`, and the separate agent-panel
  render path.
- `src/ui/mobile.rs` — consumes `agent_panel_entries`, `agent_panel_entries_from`, and
  `agent_panel_status_key`; the merged builder must keep these consumers working.
- `src/ui.rs` — sidebar area allocation and the removed section split.
- `src/app/state.rs` — removes `sidebar_section_split` from `AppState` and `ClientViewState`;
  reworks `workspace_card_areas` into merged row areas.
- `src/app/input/sidebar.rs`, `src/app/input/mouse.rs`, `src/app/input/mod.rs`,
  `src/app/input/terminal.rs` — divider drag removal and hit-testing against merged rows.
- `src/app/actions.rs`, `src/app/mod.rs` — split restore/seed paths.
- `src/persist/snapshot.rs`, `src/persist/restore.rs`, `src/persist/io.rs` — removes the
  persisted `sidebar_section_split` field.
- `src/server/headless.rs`, `src/server/headless/live_handoff.rs`, `src/server/handoff.rs` —
  removes the handoff mirrors.
- `openspec/specs/everyday-ui-preferences/spec.md` — restated agent-sort requirement.
- `docs/next/website/src/content/docs/`, `docs/next/CHANGELOG.md` — the removed configuration
  and the new layout.
- Operators who set a sidebar section split lose that setting. The value is ignored on restore
  and the sidebar renders one list.

## Preconditions

- premise: the two sections already agree on hierarchy, so the merge removes a projection rather
  than inventing one — verified: `AgentPanelSort::Spaces` is the default and its label is
  `grouped`, `src/app/state.rs:1111-1125` and `src/app/state.rs:2593` @ shepherd@cfb8358d
- premise: a workspace-to-pane tree builder already exists and is proven — verified:
  `AppState::navigator_rows_from`, `src/app/actions.rs:381`, with tab nodes emitted only for
  multi-tab workspaces per `navigator_rows_show_tab_nodes_only_for_multi_tab_workspaces`,
  `src/app/actions.rs:3697` @ shepherd@cfb8358d
- premise: agent rows already carry their own space and tab coordinates, so a flat list can
  activate the right tab — verified: `AgentPanelEntry { ws_idx, tab_idx, pane_id, .. }`,
  `src/ui/sidebar.rs:26-42` @ shepherd@cfb8358d
- premise: removing the persisted split field will not break existing session files — verified:
  `SessionSnapshot` carries no `deny_unknown_fields` and every optional field is `#[serde(default)]`,
  `src/persist/snapshot.rs:16-29`, so an unknown key in an older snapshot is ignored
  @ shepherd@cfb8358d
- premise: no protocol bump is required for the removed `ClientViewState` field — verified:
  source `PROTOCOL_VERSION` is 19 (`src/protocol/wire.rs:16`) and the latest released tag
  `v0.7.5` carries 17, so the source protocol is already greater than the released protocol and
  CLAUDE.md's bump condition does not apply @ shepherd@cfb8358d
- premise: the split reaches further than configuration and must be removed everywhere — verified:
  `sidebar_section_split` resolves in 15 files spanning `src/app/`, `src/persist/`, `src/server/`,
  and `src/ui/` @ shepherd@cfb8358d
- premise: space collapse state is already persisted and can carry the merged list's collapse —
  verified: `SessionSnapshot.collapsed_space_keys`, `src/persist/snapshot.rs:27-28`
  @ shepherd@cfb8358d
- premise: `anchor-chrome-to-sidebar` rewrites the same render path and must land first —
  verified: its `## Impact` names `src/ui.rs`, `src/ui/sidebar/`, and `src/app/state.rs`,
  `openspec/changes/anchor-chrome-to-sidebar/proposal.md` @ shepherd@cfb8358d

## Decisions

- Tree depth — chosen: fully flat, space then agents, with no tab rows at any depth; rejected:
  matching the navigator's space/tab/pane depth, and a flat list carrying a per-row tab badge,
  both shown as rendered sidebar mockups before the choice; decided-by: leo
- Organising model — chosen: the project is the unit an operator works in and its agents are what
  they switch between, so an agent row is the tab affordance; rejected: keeping tabs as an
  independent grouping layer visible in the sidebar; decided-by: leo
- `sidebar_section_split` disposition — chosen: remove it cleanly, including the config field, the
  client-view field, and the persisted snapshot field; rejected: preserving both layouts behind a
  `[ui.sidebar] mode` key, and retaining the key as an ignored no-op, the first because it ships
  two sidebar layouts to maintain and the second because a key that silently does nothing is a
  truthful-guidance defect; decided-by: leo
- Empty state — chosen: a space with no detected agent renders an explicit empty-state row;
  rejected: rendering the space row alone, because absence of a child row is indistinguishable
  from a collapsed space; decided-by: default
- Structural scope — chosen: treat "an agent is a tab" as a navigation and presentation model,
  leaving `Tab` free to hold splits and multiple panes; rejected: enforcing a one-agent-per-tab
  invariant in the runtime, because that changes session identity and persistence and belongs in
  its own proposal; decided-by: default
- Sequencing — chosen: depend on `anchor-chrome-to-sidebar` rather than merging into it; rejected:
  folding both into one change, because re-anchoring chrome geometry and replacing the sidebar's
  row model are independently reviewable; decided-by: default

## Done Means

- The sidebar renders one list: each space row is followed directly by its own agent rows, with
  no second section and no divider.
- Selecting an agent row activates that agent's tab and focuses its pane, from any space.
- A space running no detected agent shows an explicit empty-state row beneath it.
- Worktree children still group under their parent space and still collapse from the chevron.
- The agent sort control still reads and writes one persisted source, and reordering the list
  reorders every space's agents consistently.
- A session file written before this change restores without error, and the sidebar ignores any
  section-split value it carries.

## Testing

- A row-model test asserting the merged list emits space rows each followed by their own agent
  rows, in space order, with no tab rows at any depth.
- A test asserting an agent row activates the correct tab and pane when its space is not active.
- A test asserting a space with no detected agent emits exactly one empty-state row, and that the
  row is not selectable as an agent.
- A test asserting worktree children keep their indentation and their collapse chevron inside the
  merged list.
- A test asserting shortcut numbers are assigned by visible position across the merged list.
- A scroll-metrics test over a merged list taller than the sidebar, and a hit-test asserting
  clicks map to the correct row after scrolling.
- A restore test loading a snapshot that still carries `sidebar_section_split` and asserting it
  is ignored without error.
- A `ClientViewState` field-name test updated to prove the field is gone from the client
  projection.
- Mobile tests asserting `agent_panel_entries` consumers still resolve after the merge.
- `just check` passes.
