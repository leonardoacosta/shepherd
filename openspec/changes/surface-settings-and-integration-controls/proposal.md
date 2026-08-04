## Why

Settings clips sections, integration targets, and operation results at supported terminal sizes while leaving existing per-target operations and common UI preferences inaccessible. Shepherd also exposes ambiguous runtime-authority booleans and has no safe way to reach advanced configuration from a remote-capable TUI, so the visible Settings surface cannot yet provide a complete or truthful control path.

These outcomes remain one coordinated change because responsive selection/scroll state, integration observations, native preferences, and the advanced action all modify the same Settings state/render/input boundary. The tasks stage that shared foundation first rather than creating overlapping active feature claims.

## What Changes

- Replace fixed Settings tabs and unscrolled content with pure, shared navigation/viewport geometry that keeps the selected section and row visible at 40x20, 64x20, and 80x24.
- Turn Integrations into a selectable per-target manager with install/update/uninstall, explicit uninstall confirmation, version/path details, retained bulk install, and reachable bounded operation output.
- Add a neutral server-owned state-source projection to pane and agent API models, distinguishing screen-derived state from reported state and exclusive lifecycle authority from mixed evidence; consume it as descriptive integration observations without inventing health.
- Surface stable visual and interaction preferences with keyboard/mouse parity, comment-preserving single-key persistence, live reload, and explicit next-launch labeling where live application is impossible.
- Add a server-owned advanced `config.toml` editor bridge that opens the resolved server file in an editor pane, reloads only valid edits after exit, preserves invalid content for correction, and reports actionable diagnostics without deleting or wholesale-rewriting the file.
- Keep arbitrary token/style composition, a 153-field form, an embedded TOML editor, keybinding editing, and red/green integration health outside this change.

## Capabilities

### New Capabilities

- `responsive-settings-navigation`: Width-aware section navigation, shared row geometry, scroll behavior, and render/input parity.
- `integration-settings-management`: Reachable per-target lifecycle controls, status details, confirmation, results, and truthful runtime observations.
- `agent-state-source`: Neutral server/API projection of the evidence source and authority used for current agent state.
- `everyday-ui-preferences`: Native Settings controls for the selected stable visual and interaction preferences.
- `advanced-config-editing`: Server-local editor-pane access to the resolved configuration with validated reload and correction behavior.

### Modified Capabilities

None. The existing `configurable-chrome` requirements remain valid; this change adds controls around them without weakening their behavior.

## Impact

- TUI/client state: Settings section/row identity, selection, scrolling, hit-testing, confirmation, feedback, and editor-pane presentation remain client presentation concerns.
- Runtime/server state: terminal authority projection, integration operations, resolved config ownership, editor process lifecycle, validation, reload, and result events remain neutral server/runtime facts exposed through the JSON API when practical.
- Wire/API: PaneInfo and AgentInfo gain the same typed state-source projection; an editor operation/result contract may extend the schema. Compare protocol version 19 against the latest released protocol before changing it; do not bump when source is already ahead.
- Persistence: native controls use existing atomic comment-preserving single-key writes. The advanced editor edits the actual file and never replaces valid runtime state with invalid TOML.
- Documentation/generated consumers: next configuration/API references, schema, and TypeScript client are updated where contracts change; stable docs remain untouched.
- Dependencies: no dependency should be added unless existing parsing, process, and geometry utilities are proved insufficient.
- Issue linkage: not applicable because this repository has no `.beads` store and the user requested a fork-local feature queue from recorded research.
- Base and baseline:
  - base-commit: shepherd@062955ae513d4b0e3281e957043b190bdfd90ee6
  - dirty-baseline: untracked `improvements.md` is user-requested advisory research and is excluded from this feature.
- touches: `src/ui/settings.rs`, `src/ui/scrollbar.rs`, `src/app/state.rs`, `src/app/input/settings.rs`, `src/app/input/mouse.rs`, `src/app/input/navigate.rs`, `src/app/input/mod.rs`, `src/app/config_io.rs`, `src/app/api.rs`, `src/app/api/integrations.rs`, `src/app/agents.rs`, `src/app/runtime_mutations.rs`, `src/app/mod.rs`, `src/api/schema.rs`, `src/api/schema/agents.rs`, `src/api/schema/panes.rs`, `src/api/schema/integrations.rs`, `src/api/schema/server.rs`, `src/api/schema/events.rs`, `src/api/schema/response.rs`, `src/api/schema/tests.rs`, `src/integration/types.rs`, `src/integration/registry.rs`, `src/terminal/state.rs`, `src/platform/mod.rs`, `src/platform/linux.rs`, `src/platform/macos.rs`, `src/platform/windows.rs`, `src/platform/fallback.rs`, `src/config/model.rs`, `src/config/io.rs`, `src/config/sidebar.rs`, `src/ui/sidebar.rs`, `src/app/input/sidebar.rs`, `src/protocol/wire.rs`, `src/server/headless.rs`, `docs/next/api/shepherd-api.schema.json`, `clients/ts/src/index.ts`, `clients/ts/test/client.test.ts`, `clients/ts/test/generate-types.test.ts`, `docs/next/CHANGELOG.md`, `docs/next/website/src/content/docs/configuration.mdx`, `docs/next/website/src/content/docs/ja/configuration.mdx`, `docs/next/website/src/content/docs/zh-cn/configuration.mdx`, `docs/next/website/src/data/config-reference.json`, `scripts/test_config_reference_check.py`, `scripts/test_socket_api_reference_check.py`

## Preconditions

- Refresh target membership, config-key inventory, active OpenSpec claims, protocol version versus the latest released tag, and the dirty baseline before implementation.
- Treat this as a broad refactor-risk change: run the project-required roundtable and name characterization tests for render purity, Settings selection, workspace/tab/pane identity, overlay exit, and runtime authority before moving code.
- Establish one typed row/viewport model used by rendering and hit-testing before adding new rows; later tasks depend on that foundation.
- Preserve unrelated user changes and use `just` recipes for validation.

## Decisions

- Use a pure Settings viewport model with overflow controls or a compact selected-section label, and reuse shared scrollbar geometry. This is `decided-by: user` through the recorded recommendation. Rejected: another fixed-width tab or separate render/mouse calculations.
- Model Integrations as rows keyed by `IntegrationTarget`; preserve bulk install as a secondary action and require explicit confirmation before uninstall. This is `decided-by: user`. Rejected: a passive status board or Settings-specific runtime operations.
- Expose state evidence as a typed neutral projection shared by PaneInfo and AgentInfo: `screen` or `reported { source, authority }`, where authority distinguishes exclusive full-lifecycle reporting from evidence that may yield to screen detection. Keep session identity/source separate and never expose session identifiers in aggregate UI copy. This is `decided-by: user`. Rejected: installed-equals-healthy, `screen_detection_skipped` as health, or report-age traffic lights.
- Display integration observations descriptively: `reporting state in N panes`, `session identity available in N panes`, or `installed; not currently observed`. This is `decided-by: default` from the neutral model. Rejected: `broken`, `healthy`, or freshness claims without a heartbeat contract.
- Put pane borders, pane gaps, hide-single-tab-bar, Agent sort, and other selected visual values under Display; add one Behavior section for close confirmation, naming prompts, copy-on-select, and scroll speed. Both the sidebar header and Settings write the same `agent_panel_sort` key, with the header labeled `sort: grouped` or `sort: priority`. This is `decided-by: user`. Rejected: independent duplicated sort state.
- Native controls only cover pane borders/gaps, single-tab-bar visibility, close confirmation, tab/workspace naming prompts, copy-on-select, mouse scroll speed, and Agent sort. This is `decided-by: user`. Rejected: keybinding, mouse-capture, host-cursor, passthrough, startup-only sidebar, custom theme, token composer, and arbitrary advanced/experimental controls.
- Add a neutral server API operation that opens the resolved server config in a server-owned editor pane and returns pane identity; the requesting TUI decides to present that pane as an overlay. On exit, the server validates before reload, keeps the last valid runtime config on failure, retains the edited invalid file, and emits/returns diagnostics so the same file can be reopened. This is `decided-by: default` to honor remote clients and the runtime/client boundary. Rejected: client-local paths, a TUI-private socket command, a raw editor widget, or deleting the real config as temporary scrollback cleanup does.
- Put an `edit config.toml` action at the end of Behavior as the advanced escape hatch; it is an action row, not a mirrored 153-key form. This is `decided-by: default` because Behavior already owns interaction controls and the responsive foundation makes the row reachable. Rejected: hiding the bridge behind an undiscoverable shortcut or adding a separate fixed-width section.
- Reuse platform-specific argv construction without interpolating the config path into an unquoted shell fragment. Allow one active config editor operation at a time and return its existing pane identity on duplicate requests. This is `decided-by: default` for file/process safety. Rejected: concurrent editors with last-writer-wins reloads.

## Done Means

- All sections, integration targets, selected preference rows, and operation failures are reachable at 40x20, 64x20, and 80x24; keyboard and mouse resolve through identical row geometry.
- Per-target install/update/uninstall and bulk install refresh status, show current/expected version and path where applicable, confirm destructive uninstall, and retain every result in reachable detail.
- PaneInfo and AgentInfo agree on state source for the same terminal, lifecycle and session-only integrations receive distinct truthful copy, and absent observation is never reported as failure or health.
- Every selected native preference persists one key without clobbering comments, applies live where supported, labels any next-launch behavior, and keeps sidebar/Settings sort state unified.
- Advanced editing targets the server's resolved config, preserves argv boundaries and the file itself, reloads valid edits after exit, preserves last valid runtime state on invalid TOML, and lets the user immediately reopen/correct with diagnostics.
- Focused Settings, integration, terminal/API, editor-overlay, config, schema/client, docs, identity-invariant, and platform tests plus `just check` pass.

## Testing

- Run `just test-one settings`; expected result: 40x20, 64x20, and 80x24 navigation, selection, scrolling, hit-testing, preference, integration, confirmation, result-detail, and malformed-config cases pass.
- Run `just test-one integration`; expected result: per-target and bulk operations, version/path detail, status refresh, partial failure, uninstall confirmation plumbing, and observation aggregation pass.
- Run `just test-one agent_state_source`; expected result: screen, mixed report, full-lifecycle report, session-only, expiry/clear, PaneInfo/AgentInfo parity, and no-session-id aggregation cases pass.
- Run `just test-one config_editor`; expected result: server path ownership, editor argv boundaries, duplicate-open behavior, overlay exit, valid reload, invalid-file retention, last-valid runtime preservation, diagnostics, and Linux/macOS/Windows platform contracts pass.
- Run `just test-one config_io && just test-one sidebar`; expected result: one-key comment preservation, live application, malformed fallback, unified sort persistence, and clarified header behavior pass.
- Run `python3 -m unittest scripts.test_config_reference_check scripts.test_socket_api_reference_check && bun --cwd clients/ts test`; expected result: generated config/API references and TypeScript schema/client tests pass.
- Run `openspec validate surface-settings-and-integration-controls --strict --no-interactive && git diff --check`; expected result: both commands exit 0.
- Run `just check`; expected result: formatting, nextest, Windows-target lint, generated assets/clients, docs, and maintenance suites pass.
