## Context

The fork introduced `[ui.topbar]` and `[ui.dock]` in commit `b85ea7c5`, but the implementation stopped at a partial presentation layer:

- `src/ui/topbar.rs` iterates configured rows into one span list, resolves only `workspace` and workspace `$custom`, and discards configured token styles.
- `src/ui.rs` always reserves one topbar row and reserves the configured dock region whenever dock enablement is true.
- `src/ui/dock.rs` silently renders nothing when the active workspace lacks a valid dock pane/runtime, leaving the reserved region blank.
- `SettingsSection` and its keyboard/mouse/render paths expose pane border labels but not topbar/dock configuration or dock entrypoints.
- `dock_rect` is render-only. Terminal key and mouse routing target ordinary active-tab panes, so an interactive dock such as Wavetui cannot be operated in place.
- A dock process is backed by an ordinary workspace tab. That tab appears in desktop/mobile/navigator tab surfaces and is captured for disk restore even though `DockPaneRecord` ownership is not persisted.

The design must preserve Shepherd's pure render boundary, keep presentation state out of the wire protocol, reuse the existing plugin pane API, and protect workspace/tab/pane identity and restore behavior.

## Goals / Non-Goals

**Goals:**

- Make desktop chrome discoverable and configurable from the existing Settings language.
- Make topbar row/token behavior match the documented configuration contract.
- Make the dock occupy space only for a live active-workspace pane.
- Make the dock interactive in place without replacing the main active tab.
- Keep the dock backing tab out of ordinary TUI tab navigation and restore snapshots.
- Document disabled-by-default setup and the separate metadata-producer boundary.

**Non-Goals:**

- Mobile topbar/dock rendering.
- A full graphical editor for arbitrary topbar token rows.
- A generic plugin action launcher or automatic execution of plugin code.
- External metadata-producer startup/supervision.
- A wire-protocol, plugin manifest schema, dependency, or persisted snapshot format version change.
- Hiding the dock backing tab from neutral tab/pane API responses during its live process lifetime.

## Decisions

### 1. Resolve topbar rows through a pure shared token model

Extract or expose the Agent-sidebar row resolver and styling/separator helpers so topbar code consumes the same `AgentSidebarToken` semantics rather than maintaining a second incomplete matcher. Build a pure topbar context from:

- active workspace name for `workspace`;
- the active main tab's focused pane for state, tab, pane, agent, and terminal-title built-ins;
- a merged custom-token map where active workspace metadata wins and focused-pane metadata fills missing keys.

The resolver returns only nonempty rows. `compute_view()` derives the topbar height from that resolved row count and clamps it to `area.height - 1`; `render_topbar()` recomputes the same pure rows and draws one `Line` per row with the panel background. Render remains mutation-free.

Alternatives rejected:

- `SpaceSidebarToken`: semantically tidy for a workspace header, but it removes the already documented active-Agent vocabulary and does not serve pane/agent context.
- A dedicated new token enum: duplicates parsing, styling, and validation and creates another public configuration surface.
- Preserving a fixed single row: contradicts `rows = [[...], [...]]` and hides configured data.

### 2. Model dock availability separately from dock enablement

Introduce pure helpers that resolve the active workspace's dock record, backing pane, terminal ID, and runtime. Treat the record's pane ID as authoritative and locate its current tab instead of trusting a cached tab index after tab close/reorder operations. Geometry is reserved only when all are live and `dock_enabled` is true. A stale record is non-renderable and does not consume space; the next open path removes/reconciles it before applying the one-dock-per-workspace occupied check.

New dock opens while disabled return a stable `plugin_dock_disabled` error. Disabling an already-running dock only hides it and clears client dock focus; it does not kill the process. Re-enabling reveals the same live runtime. Pane exit, plugin record removal, workspace removal, or runtime loss clears effective occupancy and focus.

`DockConfig.size` accepts values of at least three cells, which leaves one inner row/column inside the existing border. Viewport geometry continues to clamp larger values while preserving one main-content row/column. If the available edge cannot fit both the minimum bordered dock and one main-content cell, the dock rectangle remains empty and the runtime is not resized until it fits. The existing mutation phase resizes the dock terminal to that inner rectangle whenever the resolved geometry changes; render only consumes the computed rectangles and runtime snapshot.

Alternatives rejected:

- Always reserving the configured region: reproduces the blank-space failure.
- Terminating on disable: turns a presentation toggle into destructive process control.
- Auto-enabling on pane open: makes CLI/API behavior mutate persistent user preferences implicitly.

### 3. Add client-only auxiliary dock focus and input routing

Add explicit dock-focus state to `ClientViewState` and the existing per-client projection plumbing. It is valid only for the current desktop workspace while the dock is enabled and live. It does not change `Workspace.active_tab` or the main tab's focused pane.

`compute_view()` publishes a dock pane hit target/inner rectangle separately from ordinary `view.pane_infos`. Mouse-down in that target focuses the dock; mouse/key/paste forwarding uses the dock's backing pane and runtime with coordinates translated into its inner rectangle. Existing prefix/global shortcuts remain intercepted before terminal bytes are sent. Key press/release ownership continues to use `TerminalInputTarget`, so a key pressed in the dock is released to the same still-live terminal even if presentation focus changes or the dock is hidden. Runtime loss drops the release because no valid press owner remains; invalid dock focus never redirects it to another pane. Host mouse-capture policy includes a live dock runtime that requests mouse reporting so the first dock interaction is not lost.

Mouse-down on an ordinary main pane, direct TUI workspace/tab/pane navigation, disabling the dock, closing the dock, runtime loss, and leaving the active workspace clear dock focus. Generic API focus of ordinary main identities does not mutate per-client auxiliary focus. The dock border uses the existing focused-pane accent language so focus is visible. The topbar intentionally continues to describe the active main pane; auxiliary dock focus does not replace agent context. Terminal focus-in/focus-out signaling and shared focus events remain derived from the main pane; auxiliary dock input focus emits neither because the terminal runtime is shared across clients.

Shared `tab.focus`, `pane.focus`, and `plugin.pane.focus` requests that target the managed backing identity return `tab_not_focusable` or `pane_not_focusable` and leave the active main tab unchanged. Auxiliary focus belongs to the TUI client, so a server API request does not mutate it. Other pane/tab focus behavior is unchanged.

Alternatives rejected:

- A render-only preview: incompatible with keyboard-driven dock applications.
- Switching to the backing tab: duplicates the same runtime in main and dock regions and destroys the user's main-tab context.
- Reusing workspace pane focus: that focus is tab-owned and would deepen shared runtime/UI coupling.

### 4. Treat the dock tab as a TUI-hidden, non-restored backing container

Keep the backing tab in the current workspace model so existing pane IDs, lifecycle cleanup, events, and neutral APIs remain valid. A managed backing tab is a workspace-local single-pane container whose sole pane is the recorded pane ID. Pane-level topology changes that would split, move, swap, or apply another layout to that pane/tab are rejected before mutation; whole-tab rename/move/close remains supported and ownership is resolved from the pane ID afterward. Centralize `visible_tab_indices(ws_idx)` in `AppState`, excluding the active workspace's dock backing index. Desktop tab chrome, wheel cycling, key/index navigation, navigator rows, and mobile tab lists consume this helper. Visible fallback labels/ordinals derive from the filtered order so no ghost tab or numbering gap appears in TUI chrome.

Disk snapshot and history capture receive the per-workspace dock backing identities and omit those tabs, their pane-number entries, and their tab-number entries. `next_public_*_number` values remain monotonic; no existing identity is renumbered. Because dock launch requires an explicit user action, disk restore does not silently restart plugin code.

Unix live handoff is a separate capture policy: it retains the backing tab and runtime FD, and `HandoffManifest` carries an optional/defaulted dock-ownership record keyed by workspace ID and old pane ID. After restore remaps pane IDs, the importer locates the containing tab and rebuilds `dock_panes`; invalid or absent metadata degrades to the pre-feature ordinary-tab behavior without failing handoff. This is private handoff metadata, not the public JSON API or persisted snapshot format, so it does not bump `PROTOCOL_VERSION` or `SNAPSHOT_VERSION`.

Neutral `tab.list`, `tab.get`, `pane.list`, `pane.get`, and live events continue to expose the backing container. Tab move/rename/close retain their existing identities and use stable pane lookup for dock reconciliation; closing still performs ordinary runtime cleanup. Direct tab/pane/plugin-pane focus of the managed backing identity is rejected as nonfocusable. No schema changes are required.

Alternatives rejected:

- Persisting `DockPaneRecord`: requires a snapshot format/migration plus an automatic external-code restart policy.
- Moving dock panes wholly outside Workspace: breaks the current pane API's workspace/tab identity contract and is a larger server/runtime migration.

### 5. Expand Pane labels into a scrollable Display Settings section

Rename the internal `PaneLabels` section to `Display` and retain its position, avoiding a seventh fixed-width tab. A pure dynamic row model supplies:

1. agent border labels;
2. topbar enabled;
3. a nonselectable configured-row summary/hint;
4. dock enabled;
5. dock side;
6. dock size with `-`/`+` and mouse hit targets;
7. zero or more eligible dock pane entrypoints.

Keyboard and mouse paths consume the same row IDs/actions. Boolean and side changes save immediately through `config_io` and live reload. Size changes clamp to the valid minimum. Dock entrypoints are sorted by plugin name/title/IDs and include only enabled plugins whose effective platform policy supports the host and whose pane placement is `dock`. Enter/click explicitly invokes the existing plugin pane-open handler for the active workspace while preserving the current active main tab. Success closes Settings to reveal and focus the dock; failure stays in Settings and surfaces a bounded diagnostic. Empty, disabled, and occupied states render explanatory rows instead of silent absence.

The platform-eligibility helper is shared with the API manifest policy rather than duplicated in UI code.

Alternatives rejected:

- A new Chrome tab: overflows the fixed Settings tab width and fragments presentation settings.
- A full token-row editor: substantially larger input/config editing work; the row summary and docs preserve advanced TOML control.
- Invoking plugin actions by title: actions are not a one-to-one dock contract, whereas typed pane manifests are.

### 6. Keep documentation and default configuration on the unreleased path

Add disabled `[ui.topbar]` and `[ui.dock]` example sections with explanatory comments to `DEFAULT_CONFIG` in `src/main.rs`, plus a test that parses the complete emitted text through `Config`. Update only `docs/next` configuration pages/translations, config-reference data, and changelog. Explain desktop-only behavior, token sources, explicit dock opening, auxiliary focus, disk-snapshot non-restart, live-handoff preservation, and the separate external metadata-producer lifecycle.

## Risks / Trade-offs

- **[Risk] Hidden backing-tab filtering drifts across TUI surfaces.** → Centralize visible indices and cover desktop tabs, key/index/wheel navigation, navigator, mobile switcher, fallback numbering, and close/workspace transitions with characterization tests.
- **[Risk] Cached dock tab indices hide or render the wrong tab after a preceding tab closes.** → Resolve ownership from the stable pane ID, derive the current containing tab, and test insertion, reorder, and closure on both sides of the dock tab.
- **[Risk] Dock focus survives invalid state and sends input to the wrong terminal.** → Validate focus against active workspace, enablement, record, pane, and runtime at every routing boundary; clear it on every lifecycle/navigation transition; preserve press/release target identity.
- **[Risk] Snapshot exclusion corrupts active indices or public-number maps.** → The active main tab remains non-dock by invariant; filter without renumbering live identities and run `AppState::assert_invariants_for_test()` / `Workspace::assert_invariants_for_test()` over adversarial identity state before and after capture.
- **[Risk] Disk-snapshot filtering accidentally drops a live dock during server handoff.** → Use explicit disk and handoff capture policies, transfer optional ownership metadata with the runtime FD, and test both backward-compatible metadata absence and pane-ID remapping.
- **[Risk] Topbar compute/render resolution diverges.** → Use one pure resolver for both row count and render content and test the same state through both paths.
- **[Risk] Dynamic Display rows make keyboard and mouse selection disagree.** → Generate both rendering and hit testing from one row model and test registry refresh, zero candidates, multiple candidates, and selection clamping.
- **[Trade-off] Live APIs still expose the backing tab.** → Preserve current protocol and identity semantics now; document this as a neutral runtime detail and revisit only with the broader server/client migration.

## Migration Plan

1. Add characterization coverage and complete the project-required roundtable before moving state or routing code.
2. Land the pure topbar resolver and occupancy helpers with focused geometry tests.
3. Add hidden-tab projection, disk-snapshot exclusion, live-handoff ownership transfer, and dock focus/input behind the existing disabled-by-default dock config.
4. Add Display Settings controls and explicit pane opening.
5. Add default-config and unreleased documentation, then run the full repository gate.

No stored configuration migration is required. Existing valid `ui.topbar` rows retain their syntax; existing dock size values below three become a clear config diagnostic instead of producing an unusable bordered region. Rollback consists of disabling both sections; no server/schema downgrade is involved.

## Open Questions

None. Automatic metadata-producer lifecycle and API-level hiding of dock backing tabs are explicitly separate future features.

## Apply baseline and roundtable record

- Refreshed 2026-08-03 at `4d8e5194a53ba404113561b592ce29efaa632d8e`; the task worktree was clean, the stamped base `9bd33861` remained reachable, `origin/master` had no commits absent from the task branch, and no active OpenSpec change overlapped this change's implementation paths.
- Geometry characterization: `topbar_resolved_rows_drive_geometry_and_render_without_render_mutation`, `topbar_clips_resolved_rows_to_preserve_one_desktop_body_row`, `dock_enabled_without_live_runtime_reserves_no_geometry`, `dock_hides_when_terminal_cannot_fit_minimum_bordered_dock_and_body`, and `dock_resolver_tracks_pane_after_preceding_tab_close_insert_and_reorder`.
- Identity/persistence characterization: adversarial visible-tab projection with both invariant helpers, disk structural/history capture in every dock position, handoff pane-ID remapping plus invalid optional metadata, protected backing topology, and cleanup through tab/pane/plugin/workspace/runtime removal.
- Input characterization: dock press/repeat/release ownership, paste routing, first-click mouse capture and inner-coordinate routing, popup precedence, TUI-only focus clearing, API nonfocusable rejection, and per-client projection freeze tests.
- Settings/config characterization: minimum-size parse/live-reload rollback, one shared Display row model for render/key/mouse, comment/sibling-preserving writes, deterministic platform-eligible candidate ordering, explicit single launch, default-config parsing, locale parity, and Windows compile exposure.
- Roundtable outcome: zero unresolved blockers after adopting the minimum-fit, single-pane topology, per-client focus, paste/mouse-capture, shared-focus-event, and separate disk/handoff policies above.
