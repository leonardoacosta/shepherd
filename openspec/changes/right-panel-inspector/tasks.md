## 1. Restructure the right-panel configuration

- [ ] 1.1 In `src/config/sidebar.rs`, remove `rows` from `RightPanelConfig` and add a `tabs` field
      typed as an ordered list of a new tab-name enum. Keep `enabled` and `width`. Add
      `min_width`/`max_width` clamping matching the sidebar's treatment.
- [ ] 1.2 Add a per-tab configuration type for the proposals tab holding its own token rows, typed
      against a proposals-specific token enum. Do not extend `AgentSidebarToken`.
- [ ] 1.3 Wire `[ui.right_panel.<tab>]` sub-tables in `src/config/model.rs` so each declared tab
      deserializes its own rows.
- [ ] 1.4 Add config tests: the sub-table parses and round-trips; an unset panel defaults to
      disabled; an enabled panel with an empty `tabs` list is valid and reserves nothing; an
      unknown tab name in `tabs` is a named error; a token from another tab's vocabulary is a
      named error citing the token and the tab.
- [ ] 1.5 Run `cargo nextest run config` and paste the passing output.

## 2. Move panel geometry from resolved rows to configured tabs

- [ ] 2.1 In `src/ui.rs`, gate `right_panel_w` on the panel being enabled with a non-empty `tabs`
      list, replacing the `resolved_right_panel_rows(app).is_empty()` check. Keep the existing
      clamp that preserves at least one main-content column.
- [ ] 2.2 In `src/ui/topbar.rs`, remove `resolved_right_panel_rows` and `render_right_panel`. Leave
      `resolved_chrome_rows` and the topbar's use of it untouched.
- [ ] 2.3 In `src/app/state.rs`, remove the resolved right-panel rows field and add the selected
      tab and per-tab scroll offsets as client presentation state.
- [ ] 2.4 Add geometry tests: width is reserved for a configured tab whose content resolves
      nothing; an empty tab list reserves nothing; the panel spans the full terminal height; both
      side panels on a narrow terminal leave one main-content column.
- [ ] 2.5 Run `cargo nextest run ui` and paste the passing output.

## 3. Build the tab surface

- [ ] 3.1 Add the width-tier resolver: below the width the declared tab labels require, render a
      header naming the selected tab and its position; at or above it, render the tab labels in
      declared order with the selected one distinguished. Derive the threshold from the declared
      label set, not a hardcoded number.
- [ ] 3.2 Render the selected tab's body through `sidebar::resolved_token_spans`. Do not add a
      second renderer.
- [ ] 3.3 In `src/app/input/mouse.rs`, add hit-testing for tab labels and the body region. A click
      on a label selects that tab; a scroll in the body scrolls the selected tab. Neither changes
      the active workspace, tab, or focused pane.
- [ ] 3.4 Keep scroll offset per tab so returning to a tab restores its position.
- [ ] 3.5 Add width-tier tests asserting the header renders below the threshold, the tab bar at and
      above it, and the selected tab is identifiable in both.
- [ ] 3.6 Add hit-test tests for tab selection and body scrolling. Seed the computed frame entries,
      not the panel rectangle alone — seeding a rectangle alone fakes a computed frame and the
      hit-test then measures a different list than the render produced.
- [ ] 3.7 Add an input test asserting a keystroke with the panel visible and a tab selected reaches
      the focused pane and leaves the selected tab and scroll position unchanged.
- [ ] 3.8 Run `cargo nextest run` for the `ui` and `app::input` modules and paste the passing
      output.

## 4. Widen the proposals domain type

- [ ] 4.1 In `src/workspace/project_status.rs`, widen the proposals half of the snapshot to carry
      per-proposal items — name, completed tasks, total tasks — alongside the existing derived
      open and in-progress counts. Keep parsing pure and in this module.
- [ ] 4.2 Update `parse_proposal_counts` to retain the per-change names it currently discards.
      Preserve the existing derivation of `in_progress` from task counts.
- [ ] 4.3 In `src/app/project_status_refresh.rs`, parse the wider shape. Leave demand resolution,
      per-checkout deduplication, refresh cadence, off-render-path execution, and per-provider
      fail-open unchanged.
- [ ] 4.4 Extend the proposals demand predicate so a configured proposals tab enables the provider,
      in addition to a configured space row consuming the `proposals` token.
- [ ] 4.5 Add parser tests over captured `openspec list --json` bodies producing per-proposal
      items, plus malformed-body, empty-array, and non-zero-exit cases resolving to no value.
- [ ] 4.6 Add a demand test asserting no refresh starts when neither a space row nor a tab consumes
      proposal status, and one asserting a configured tab alone enables it.
- [ ] 4.7 Add a test asserting a space row and the tab consuming the same checkout produce one
      refresh, not two.
- [ ] 4.8 Run `cargo nextest run project_status` and paste the passing output.

## 5. Render the proposals tab

- [ ] 5.1 Resolve the proposals token vocabulary into the shared row renderer, supplying the
      per-proposal items as the row context.
- [ ] 5.2 Render the summary line and one row per proposal, honouring the tab's configured rows.
- [ ] 5.3 Add render tests: proposals list with task counts; an empty checkout renders no entries,
      no placeholder, no error; an absent token elides it and its separator; a fully unresolved
      row is omitted.
- [ ] 5.4 Run `cargo nextest run` for the `ui` module and paste the passing output.

## 6. Apply the spec changes

- [ ] 6.1 Apply the modified `configurable-chrome` requirement so the right panel's geometry
      follows configured tabs rather than resolved content.
- [ ] 6.2 Apply the modified dock-versus-chrome-panel requirement so it forbids terminal input
      routing and terminal hosting while permitting pointer interaction.
- [ ] 6.3 Apply the added `right-panel-inspector` requirements.
- [ ] 6.4 Run `openspec validate right-panel-inspector --strict --no-interactive` and paste the
      clean output.

## 7. Document the change on the unreleased path

- [ ] 7.1 Replace the `[ui.right_panel] rows` documentation with the per-tab sub-table shape in
      `docs/next/website/src/content/docs/configuration.mdx` and its `ja` and `zh-cn` siblings.
- [ ] 7.2 Update `docs/next/website/src/data/config-reference.json` for the restructured keys.
- [ ] 7.3 Update the `docs/next/CHANGELOG.md` entry so it describes the inspector rather than the
      token strip. State that token rows on the right edge are replaced by per-tab rows and that
      `[ui.topbar]` renders the same vocabulary for anyone who wants a token strip.

## 8. Verify the complete feature

- [ ] 8.1 Run `just check` and paste the passing output.
- [ ] 8.2 Start the binary against a workspace with `tabs = ["props"]` at the default width of 24.
      Capture rendered evidence showing the cycling header and the checkout's proposals with task
      counts.
- [ ] 8.3 Capture rendered evidence at a width that fits the tab bar, showing the tab labels and
      the selected tab distinguished.
- [ ] 8.4 Capture rendered evidence against a checkout with no `openspec/changes/`, showing the tab
      empty with no placeholder and no error.
- [ ] 8.5 With the panel visible, type into the focused pane and capture evidence that the
      keystrokes reached the pane.
