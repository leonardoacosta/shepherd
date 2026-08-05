## Why

`[ui.right_panel]` shipped as a token strip: full-height, disabled by default, rendering
`AgentSidebarToken` rows through the same resolver the topbar uses. It answers "what is the
focused pane doing" a second time, on the opposite edge. That is not what the right edge is for.

The left sidebar answers *which space, which agent*. The main area answers *what is the agent
saying*. Nothing answers *what is the state of the work* — the open change proposals, the tracked
issues, the run in flight, the diff on the branch. An operator leaves Shepherd to find out.

Two facts make a tabbed inspector cheap rather than new infrastructure:

- `src/workspace/project_status.rs` and `src/app/project_status_refresh.rs` already resolve
  proposal counts per checkout, demand-driven, deduplicated, off the render path, failing open per
  provider. The proposals tab needs a wider domain type, not a new refresh loop.
- `src/ui/sidebar.rs` already exposes `resolved_token_spans` at crate visibility, and the topbar
  and right panel both consume it. A tabbed surface whose tabs each declare token rows is the same
  renderer with a different row source.

The panel is unreleased. It is absent from the root `CHANGELOG.md` and from
`src/config/sidebar.rs` at `preview-2026-07-21-0f10e1453a7f`, appearing only on the unreleased
docs path. Its configuration shape can still change without a migration, and this is the last
change where that is true.

This proposal delivers the inspector shell and one tab. Four further tabs — tracked issues, run
status, git diff, host timers — are deliberately out of scope and land as their own changes
against the surface this one establishes.

## What Changes

- Restructure `[ui.right_panel]` so each tab is its own configuration sub-table owning its own
  token rows. The panel keeps `enabled`, `width`, and a `tabs` array that declares tab order,
  enablement, and the start tab. Panel-level `rows` is removed.
- Make the right panel a tab surface: a tab affordance, a selected tab, a scrollable body, and
  pointer hit-testing for tab selection and scrolling. The panel takes no terminal input routing
  and hosts no terminal runtime.
- Resolve the tab affordance against available width. Below the width that fits a tab bar, the
  panel renders a cycling header naming the selected tab and its position instead of clipping tab
  labels.
- Add the proposals tab: a per-checkout list of open change proposals with their task counts,
  rendered through per-tab token rows.
- Widen the proposals half of the existing project-status snapshot from counts to items. The
  refresh loop, demand resolution, deduplication, and fail-open behaviour are unchanged.
- Amend `configurable-chrome` so a chrome panel is defined by what it does not host — a terminal
  runtime and terminal input routing — rather than by accepting no interaction at all.
- Document the restructured configuration on the unreleased docs path.

The tracked-issues, run-status, git-diff, and host-timer tabs are not in this change. The tab
surface is built so each arrives as a token vocabulary plus a provider, not as a new renderer.

An operator who does not enable the right panel sees no behaviour change and pays no new process
cost. The panel remains disabled by default.

## Capabilities

### New Capabilities

- `right-panel-inspector` — a tabbed inspector on the right chrome edge whose tabs each declare
  their own token rows, resolve against available width, and accept pointer interaction without
  terminal input routing.

### Modified Capabilities

- `configurable-chrome` — the right-panel requirement moves from "renders configured token rows"
  to "hosts the inspector's tab surface", and the dock-versus-chrome-panel requirement is
  restated so it forbids terminal semantics rather than all interaction.

## Impact

- `src/config/sidebar.rs` — `RightPanelConfig` loses `rows`, gains `tabs`; five per-tab config
  types, of which only the proposals one is populated by this change.
- `src/config/model.rs` — the `[ui.right_panel]` sub-table wiring.
- `src/app/state.rs` — selected tab, tab scroll offset, and the resolved per-tab rows, all client
  presentation state.
- `src/ui.rs` — right-panel geometry now depends on configured tabs rather than resolved rows;
  the tab-surface render entry point.
- `src/ui/topbar.rs` — `resolved_right_panel_rows` and `render_right_panel` are replaced by the
  tab-surface path; the topbar's own use of `resolved_chrome_rows` is untouched.
- `src/ui/sidebar.rs`, `src/ui/sidebar/tokens.rs` — the shared row renderer gains the proposals
  token context; the Agent and Space vocabularies are unchanged.
- `src/app/input/mouse.rs` — tab-label and body hit-testing for the right panel.
- `src/workspace/project_status.rs` — `ProposalCounts` widens to carry per-proposal items.
- `src/app/project_status_refresh.rs` — parses the wider shape; demand, dedup, cadence, and
  fail-open behaviour unchanged.
- `docs/next/website/src/content/docs/configuration.mdx` and its `ja` and `zh-cn` siblings, plus
  `docs/next/website/src/data/config-reference.json` and `docs/next/CHANGELOG.md` — the
  restructured configuration on the unreleased path.
- No change to pane, tab, agent, dock, or focus behaviour. No new runtime dependency: `openspec`
  is already an optional provider, and an absent binary resolves to an empty tab exactly as it
  resolves to an absent token today.

## Preconditions

- premise: `[ui.right_panel]` is unreleased, so its configuration shape can change without a
  migration — verified: `rg -c right_panel CHANGELOG.md` returns no match, and
  `git show preview-2026-07-21-0f10e1453a7f:src/config/sidebar.rs | rg -c RightPanelConfig`
  returns no match @ shepherd@b5fc4abe
- premise: the proposals provider already exists and is demand-driven per checkout — verified:
  `parse_proposal_counts` in `src/workspace/project_status.rs:56`, driven by
  `ProjectStatusRefreshDemand` and the refresh loop in `src/app/project_status_refresh.rs`
  @ shepherd@b5fc4abe
- premise: `openspec list --json` emits per-change `name`, `completedTasks`, and `totalTasks`,
  which is the full set of fields the proposals tab renders — verified: parsed today by
  `parse_proposal_counts` against this repo @ shepherd@b5fc4abe
- premise: the shared row renderer is reachable at crate visibility and is already consumed by
  two surfaces — verified: `sidebar::resolved_token_spans` called from
  `src/ui/topbar.rs:122` @ shepherd@b5fc4abe
- premise: chrome panels draw no ratatui block border, so the panel's edge against main content
  is a single divider column — verified: no `Block::` or `Borders::` construction in
  `src/ui/sidebar.rs`, and the sidebar's divider column is computed as
  `sidebar_rect.x + sidebar_rect.width - 1` at `src/app/input/sidebar.rs:1767`
  @ shepherd@b5fc4abe
- premise: the right panel currently reserves width from resolved rows, which is the coupling
  this change replaces with configured tabs — verified: `resolved_right_panel_rows(app).is_empty()`
  gates `right_panel_w` at `src/ui.rs:246` @ shepherd@b5fc4abe
- premise: the default panel width leaves 21 content columns, which is narrower than a five-label
  tab bar — verified: `DEFAULT_RIGHT_PANEL_WIDTH = 24` at `src/config/sidebar.rs:10`; the five
  tab labels joined by single separators measure 28 columns @ shepherd@b5fc4abe
- premise: seeding only a rectangle in a UI test fakes a computed frame and makes hit-tests
  measure a different list — verified: `compute_workspace_list_areas` derives row areas from
  `agent_panel_entries`, not from `sidebar_rect` alone, `src/ui.rs:322-336` @ shepherd@b5fc4abe

## Decisions

- Per-tab configuration — chosen: each tab is its own sub-table `[ui.right_panel.<tab>]` owning
  its own token rows, with a panel-level `tabs` array declaring order, enablement, and start tab;
  rejected: a panel-level `mode = "tokens" | "inspector"` switch keeping the old rows alongside
  the new surface, and adding the inspector as a third independent chrome region beside the token
  panel and the dock; decided-by: leo
- Panel-level `rows` disposition — chosen: remove it, because the panel is unreleased and the
  token strip's job is already done by `[ui.topbar]` with the same vocabulary; rejected:
  retaining it for backward compatibility, which the Breaking Changes Policy requires be asked
  rather than assumed and which nothing shipped depends on; decided-by: leo
- Renderer shape — chosen: one shared row renderer plus one token vocabulary per tab; rejected:
  a bespoke renderer per tab, which reintroduces exactly the resolver fork that
  `anchor-chrome-to-sidebar` Decision 2 forbids; decided-by: default
- Token enum granularity — chosen: a separate token enum per tab, mirroring the existing split
  between `AgentsSidebarConfig` and `SpacesSidebarConfig`, so a token used in the wrong tab is a
  named configuration error; rejected: one extended `AgentSidebarToken` enum, where a misplaced
  token would silently elide and read as missing data; decided-by: default
- Interaction model — chosen: pointer selection and scrolling only, matching the left sidebar;
  rejected: keyboard focus routing into the panel, which is the larger amendment to
  `configurable-chrome` and would put a second consumer in front of the terminal input path;
  decided-by: default
- Narrow-width affordance — chosen: a cycling header naming the selected tab and its position
  when the width cannot fit a tab bar; rejected: clipping tab labels, which at the default width
  of 24 would render a truncated first label and no indication that other tabs exist;
  decided-by: default
- Scope of this change — chosen: the shell plus the proposals tab, so the interaction contract is
  spec-checked before four more tabs depend on it; rejected: all five tabs in one change, which
  would land a new git subsystem, a host-scoped store, and a pane-scoped store alongside the
  surface that hosts them; decided-by: default
- Tab order source — chosen: an explicit `tabs` array; rejected: implicit sub-table declaration
  order, which TOML does not preserve through deserialization and which leaves the start tab
  undefined; decided-by: default

## Done Means

- An operator can set `[ui.right_panel]` with `tabs = ["props"]` and see the open change
  proposals of the space's checkout listed in the right panel, with their task counts.
- An operator can click a tab label to change the selected tab, and scroll the tab body with the
  mouse wheel.
- At the default width of 24 the panel renders a cycling header naming the selected tab and its
  position, rather than a clipped tab bar.
- Typing while the panel is visible reaches the focused terminal pane; no keystroke is consumed
  by the panel.
- A space whose checkout has no `openspec/changes/` renders the proposals tab empty, with no
  placeholder and no error surface.
- An operator who enables no tabs observes the panel reserve no width and spawn no provider.

## Testing

- Config tests asserting the per-tab sub-tables parse, round-trip through serialization, reject
  an unknown tab name in `tabs`, and reject a token used in the wrong tab's rows by name.
- A config test asserting an unset right panel defaults to disabled and reserves no width, and
  one asserting an enabled panel with an empty `tabs` array reserves no width either.
- Geometry tests asserting the panel reserves its configured width from configured tabs rather
  than from resolved rows, that both side panels clamp to preserve one main-content column, and
  that the panel spans the full terminal height.
- Width-tier tests asserting the cycling header renders below the tab-bar threshold, the tab bar
  renders at and above it, and the selected tab is identifiable in both.
- Hit-test tests for tab selection and body scrolling, seeding the computed frame entries rather
  than the panel rectangle alone, so the hit-test measures the list the render produced.
- An input test asserting a keystroke with the panel visible and a tab selected still reaches the
  focused pane.
- Parser tests over captured `openspec list --json` bodies producing per-proposal items, plus
  malformed-body, empty-array, and non-zero-exit cases resolving to no value.
- A demand test asserting the proposals provider does not start when no configured tab consumes
  it, preserving the existing demand-driven contract.
- Render tests asserting an absent per-proposal token elides it and its separator, and that a
  fully unresolved row is omitted, matching existing token behaviour.
- `openspec validate right-panel-inspector --strict --no-interactive` passes.
- `just check` passes.
