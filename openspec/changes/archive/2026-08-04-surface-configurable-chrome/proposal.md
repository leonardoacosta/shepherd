## Why

Shepherd already contains a configurable desktop topbar and workspace plugin dock, but users cannot discover or operate most of that behavior from the TUI: topbar rows are flattened and only partially resolved, an empty enabled dock consumes terminal space, and Settings does not expose either surface. Surface the existing chrome coherently so the fork's intended UI is visible without requiring source inspection or memorized plugin commands.

## What Changes

- Turn the current pane-label Settings section into a broader Display section that exposes topbar and dock enablement, dock side and size, and explicit opening of eligible installed dock panes.
- Render topbar rows as configured, including the documented Agent-sidebar built-ins, styles, separators, missing-value elision, and workspace metadata used by existing `$custom` telemetry.
- Reserve full dock geometry only while the active workspace owns a live dock pane, and define disabled, empty, occupied, closed, stale-runtime, and workspace-switch behavior.
- Make the dock usable in place: mouse focus and terminal input target the dock without replacing the workspace's active main tab, while the dock's backing tab stays out of ordinary TUI tab chrome/navigation and disk session restore.
- Add disabled topbar and dock example sections with explanatory comments to `shepherd --default-config` and document the desktop behavior, metadata contract, and dock-pane discovery in unreleased docs and release notes.
- Keep mobile geometry, the shared wire protocol, generic plugin actions, automatic plugin execution, and external telemetry-producer lifecycle outside this change.

## Capabilities

### New Capabilities

- `configurable-chrome`: Discoverable desktop Display settings, correct configurable topbar rendering, and explicit workspace dock discovery and geometry behavior.

### Modified Capabilities

None. This repository has no existing canonical OpenSpec capability specifications.

## Impact

- Runtime/client boundary: all new selection, layout, hit-target, and visibility behavior remains TUI presentation state; dock launching reuses the existing neutral plugin pane API and installed-plugin registry. Private handoff metadata transports existing server-owned dock placement without adding a TUI-shaped public API.
- Dependencies and protocol: no dependency, public wire-schema, protocol-version, persisted-snapshot-format-version, or plugin-manifest-placement changes. Disk snapshot content intentionally omits dock execution; the private Unix live-handoff manifest gains optional dock-ownership metadata so updates still transfer a live dock.
- Documentation: only unreleased docs under `docs/next/` and the executable default-config source change; stable website docs and root release docs remain untouched.
- Issue linkage: not applicable because this repository has no `.beads` store and the user invoked a fork-local feature objective rather than a linked issue.
- Base and baseline:
  - base-commit: shepherd@9bd33861aaf9494ad57050c625e770e38a1a11ae
  - dirty-baseline: clean
- touches: `src/config/topbar.rs`, `src/config/dock.rs`, `src/ui/sidebar/tokens.rs`, `src/ui/topbar.rs`, `src/ui/dock.rs`, `src/ui/tabs.rs`, `src/ui/mobile.rs`, `src/ui/navigator.rs`, `src/ui/settings.rs`, `src/ui.rs`, `src/app/state.rs`, `src/app/actions.rs`, `src/app/input/settings.rs`, `src/app/input/terminal.rs`, `src/app/input/mouse.rs`, `src/app/input/navigate.rs`, `src/app/input/mod.rs`, `src/app/config_io.rs`, `src/app/mod.rs`, `src/app/session.rs`, `src/app/api/tabs.rs`, `src/app/api/panes.rs`, `src/app/api/plugins/mod.rs`, `src/app/api/plugins/panes.rs`, `src/persist.rs`, `src/persist/snapshot.rs`, `src/server/handoff.rs`, `src/server/headless.rs`, `src/server/headless/live_handoff.rs`, `src/main.rs`, `docs/next/CHANGELOG.md`, `docs/next/website/src/content/docs/configuration.mdx`, `docs/next/website/src/content/docs/ja/configuration.mdx`, `docs/next/website/src/content/docs/zh-cn/configuration.mdx`, `docs/next/website/src/data/config-reference.json`

## Preconditions

- Before implementation edits, refresh the stamped base, dirty baseline, active OpenSpec changes, and touched-path overlap check.
- Because the implementation crosses UI geometry, settings input, config persistence, and plugin pane state, complete the project-required broad-change roundtable and name the characterization tests that protect current workspace, tab, pane, and dock identity invariants.
- Use `just` recipes for repository validation and preserve unrelated user changes if the worktree is no longer clean.

## Decisions

- Keep `AgentSidebarToken` as the topbar configuration vocabulary. Resolve Agent built-ins from the focused pane in the active workspace, resolve `workspace` from the active workspace, and resolve `$custom` from workspace metadata first with focused-pane metadata as fallback. This preserves existing workspace telemetry while making the advertised built-ins useful. Rejected: changing the unreleased config to `SpaceSidebarToken`, which would discard the documented active-pane vocabulary.
- Reuse sidebar token styling, separator, and empty-row-elision rules. Render every resolved row separately and cap visible topbar rows so desktop content retains at least one body row. Rejected: retaining the one-row flattening, which contradicts the configured row shape.
- Treat `ui.dock.enabled` as permission and presentation configuration, not proof that a pane occupies the dock. Reserve the configured full dock size only for a valid active-workspace dock record with a live runtime. Resolve the backing tab from stable pane identity rather than trusting a stored tab index after earlier tabs close or move. Disabling hides but does not terminate an already-running dock pane; closing or losing the runtime releases geometry. Rejected: reserving a blank full-size region or killing plugin state on a display toggle.
- Treat the dock as an auxiliary interactive surface, not a second ordinary workspace tab. Keep its backing tab for existing list/get/event identity contracts, but exclude it from TUI tab bars, tab cycling/index shortcuts, navigator/mobile tab lists, and disk session/history snapshots. Opening or focusing the dock preserves the active main tab. Direct tab, pane, or plugin-pane focus requests for the managed backing identity return `tab_not_focusable` or `pane_not_focusable` so a shared API call cannot mutate client presentation focus or replace the main body. Clicking the live dock gives it client presentation focus and routes keys, mouse reports, selection, and scrollback to its pane; an outstanding key release follows its still-live press owner even after presentation focus changes, while ordinary input fails closed when the target is invalid. Clicking/navigating the main surface clears dock focus. Unix live handoff continues to transfer the backing runtime and uses optional private manifest metadata to rebind dock ownership after pane-ID remapping. Rejected: a read-only preview (the shipped Wavetui dock requires keys), switching the main surface to a duplicate rendering of the dock's backing tab, or silently restarting plugin code from a disk snapshot.
- Replace the narrow Pane labels Settings section with Display instead of adding a seventh fixed-width tab. Display owns the existing border-label setting plus topbar/dock controls, configured-row summary, and sorted enabled/platform-supported `placement = "dock"` pane choices. Opening a pane requires an explicit Enter/click action through the existing plugin pane path. Rejected: a generic plugin-action launcher or auto-opening the first plugin.
- Keep topbar and dock rendering desktop-only. Settings may describe the desktop-only behavior on narrow layouts, but `compute_mobile_view()` retains empty topbar and dock rectangles.
- Keep external metadata-producer startup and supervision as a separate feature. This change documents the metadata contract and explicit launch path but does not add startup hooks or cross-repository ownership.

## Done Means

- Display Settings exposes and persists the chrome values without clobbering unrelated config, and keyboard and mouse paths produce the same behavior.
- Multi-row topbar rendering, token sources, styles, elision, truncation, short-terminal behavior, and render purity are covered by focused tests.
- Empty, disabled, live, occupied, stale-runtime, closed, failed-open, and per-workspace dock states have deterministic API feedback and geometry with no unexplained blank region.
- A focused live dock receives keyboard and translated mouse input in place, shows its focus state, and relinquishes focus safely on disable, close, runtime loss, workspace/tab navigation, or main-pane click; its backing tab creates no TUI ghost row, is not automatically relaunched by disk restore, and remains bound to the live dock across compatible Unix live handoff.
- `shepherd --default-config`, the preview configuration docs and translations, config reference, and unreleased changelog describe the same disabled-by-default behavior.
- Strict OpenSpec validation, focused tests, docs checks, `git diff --check`, and `just check` pass with no wire-protocol or stable-doc diff.

## Testing

- Run `just test-one topbar`; expected result: all topbar token-resolution, style, row, focus, empty-value, and geometry tests pass.
- Run `just test-one dock`; expected result: all dock occupancy, disabled/open, close, stale-runtime, workspace-switch, and minimum-size tests pass.
- Run `just test-one dock_focus`; expected result: dock/main focus transitions, key release targeting, mouse coordinate translation, selection/scrollback, hidden-tab navigation, and snapshot exclusion tests pass, including adversarial workspace/tab/pane identity invariants.
- Run `just test-one settings`; expected result: all Display keyboard, mouse, persistence, live-reload, candidate-filtering, open, and feedback tests pass.
- Run `just test-one default_config`; expected result: the emitted `DEFAULT_CONFIG` parses through `Config`, contains topbar and dock sections, and leaves both disabled by default.
- Run `python3 -m unittest scripts.test_config_reference_check scripts.test_docs_translation_parity`; expected result: both suites exit 0 with no reference or locale parity drift.
- Run `openspec validate surface-configurable-chrome --strict --no-interactive && git diff --check`; expected result: both commands exit 0.
- Run `just check`; expected result: formatting, nextest, Windows-target lint, integration assets, generated client, and maintenance suites all pass.
