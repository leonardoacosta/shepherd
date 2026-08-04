## Why

Settings clips sections, integration targets, and operation results at supported terminal sizes while leaving existing per-target operations and common UI preferences inaccessible, so the visible Settings surface cannot yet provide a complete control path.

These outcomes remain one coordinated change because responsive selection/scroll state, integration target management, and native preferences all modify the same Settings state/render/input boundary. The tasks stage that shared foundation first rather than creating overlapping active feature claims.

This change owns Settings presentation only. The neutral state-source projection it once carried is owned by `expose-agent-state-source`, and the advanced editor bridge is owned by `open-advanced-config-editor`; both follow-ons declare that supersession in their own `## Impact`. Apply this change first — both follow-ons consume the Settings foundation it establishes.

## What Changes

- Replace fixed Settings tabs and unscrolled content with pure, shared navigation/viewport geometry that keeps the selected section and row visible at 40x20, 64x20, and 80x24.
- Turn Integrations into a selectable per-target manager with install/update/uninstall, explicit uninstall confirmation, version/path details, retained bulk install, and reachable bounded operation output.
- Surface stable visual and interaction preferences with keyboard/mouse parity, comment-preserving single-key persistence, live reload, and explicit next-launch labeling where live application is impossible.
- Keep arbitrary token/style composition, a 153-field form, an embedded TOML editor, keybinding editing, and red/green integration health outside this change.
- Keep the typed agent state-source projection and the advanced `config.toml` editor bridge outside this change; they are owned by `expose-agent-state-source` and `open-advanced-config-editor` respectively.

## Capabilities

### New Capabilities

- `responsive-settings-navigation`: Width-aware section navigation, shared row geometry, scroll behavior, and render/input parity.
- `integration-settings-management`: Reachable per-target lifecycle controls, status details, confirmation, and results.
- `everyday-ui-preferences`: Native Settings controls for the selected stable visual and interaction preferences.

### Modified Capabilities

None. The existing `configurable-chrome` requirements remain valid; this change adds controls around them without weakening their behavior.

## Impact

- TUI/client state: Settings section/row identity, selection, scrolling, hit-testing, confirmation, and feedback remain client presentation concerns.
- Runtime/server state: integration operations and their result events remain neutral server/runtime facts exposed through the JSON API when practical.
- Wire/API: integration status projection gains version/path detail. Compare protocol version 19 against the latest released protocol before changing it; do not bump when source is already ahead.
- Persistence: native controls use existing atomic comment-preserving single-key writes.
- Supersession: the `agent-state-source` and `advanced-config-editing` capability sections this change originally carried are withdrawn in favor of `expose-agent-state-source` and `open-advanced-config-editor`. Both follow-ons depend on this change landing first.
- Documentation/generated consumers: next configuration/API references, schema, and TypeScript client are updated where contracts change; stable docs remain untouched.
- Dependencies: no dependency should be added unless existing parsing, process, and geometry utilities are proved insufficient.
- Issue linkage: not applicable because this repository has no `.beads` store and the user requested a fork-local feature queue from recorded research.
- Base and baseline:
  - base-commit: shepherd@062955ae513d4b0e3281e957043b190bdfd90ee6
  - dirty-baseline: untracked `improvements.md` is user-requested advisory research and is excluded from this feature.
- touches: `src/ui.rs`, `src/ui/settings.rs`, `src/ui/scrollbar.rs`, `src/ui/sidebar.rs`, `src/app/state.rs`, `src/app/mod.rs`, `src/app/config_io.rs`, `src/app/api.rs`, `src/app/api/integrations.rs`, `src/app/input/settings.rs`, `src/app/input/mouse.rs`, `src/app/input/mod.rs`, `src/app/input/sidebar.rs`, `src/api/schema/integrations.rs`, `src/integration/types.rs`, `src/integration/registry.rs`, `src/config/model.rs`, `src/config/io.rs`, `src/config/sidebar.rs`, `docs/next/CHANGELOG.md`, `docs/next/website/src/content/docs/configuration.mdx`, `docs/next/website/src/content/docs/ja/configuration.mdx`, `docs/next/website/src/content/docs/zh-cn/configuration.mdx`, `docs/next/website/src/data/config-reference.json`, `scripts/test_config_reference_check.py`

## Preconditions

- Refresh target membership, config-key inventory, active OpenSpec claims, protocol version versus the latest released tag, and the dirty baseline before implementation.
- Treat this as a broad refactor-risk change: run the project-required roundtable and name characterization tests for render purity, Settings selection, and workspace/tab/pane identity before moving code.
- Establish one typed row/viewport model used by rendering and hit-testing before adding new rows; later tasks depend on that foundation.
- Preserve unrelated user changes and use `just` recipes for validation.

## Decisions

- Use a pure Settings viewport model with overflow controls or a compact selected-section label, and reuse shared scrollbar geometry. This is `decided-by: user` through the recorded recommendation. Rejected: another fixed-width tab or separate render/mouse calculations.
- Model Integrations as rows keyed by `IntegrationTarget`; preserve bulk install as a secondary action and require explicit confirmation before uninstall. This is `decided-by: user`. Rejected: a passive status board or Settings-specific runtime operations.
- Put pane borders, pane gaps, hide-single-tab-bar, Agent sort, and other selected visual values under Display; add one Behavior section for close confirmation, naming prompts, copy-on-select, and scroll speed. Both the sidebar header and Settings write the same `agent_panel_sort` key, with the header labeled `sort: grouped` or `sort: priority`. This is `decided-by: user`. Rejected: independent duplicated sort state.
- Native controls only cover pane borders/gaps, single-tab-bar visibility, close confirmation, tab/workspace naming prompts, copy-on-select, mouse scroll speed, and Agent sort. This is `decided-by: user`. Rejected: keybinding, mouse-capture, host-cursor, passthrough, startup-only sidebar, custom theme, token composer, and arbitrary advanced/experimental controls.
- Withdraw the `agent-state-source` and `advanced-config-editing` capability sections in favor of the `expose-agent-state-source` and `open-advanced-config-editor` follow-ons, which each declare that supersession in their own `## Impact`. This is `decided-by: leo` at apply time. Rejected: shipping both copies, which would implement each capability twice against the same source files.

## Done Means

- All sections, integration targets, selected preference rows, and operation failures are reachable at 40x20, 64x20, and 80x24; keyboard and mouse resolve through identical row geometry.
- Per-target install/update/uninstall and bulk install refresh status, show current/expected version and path where applicable, confirm destructive uninstall, and retain every result in reachable detail.
- Every selected native preference persists one key without clobbering comments, applies live where supported, labels any next-launch behavior, and keeps sidebar/Settings sort state unified.
- Focused Settings, integration, config, docs, and identity-invariant tests plus `just check` pass.

## Testing

- Run `just test-one settings`; expected result: 40x20, 64x20, and 80x24 navigation, selection, scrolling, hit-testing, preference, integration, confirmation, result-detail, and malformed-config cases pass.
- Run `just test-one integration`; expected result: per-target and bulk operations, version/path detail, status refresh, partial failure, and uninstall confirmation plumbing pass.
- Run `just test-one config_io && just test-one sidebar`; expected result: one-key comment preservation, live application, malformed fallback, unified sort persistence, and clarified header behavior pass.
- Run `python3 -m unittest scripts.test_config_reference_check`; expected result: the generated config reference matches the curated preference rows.
- Run `openspec validate surface-settings-and-integration-controls --strict --no-interactive && git diff --check`; expected result: both commands exit 0.
- Run `just check`; expected result: formatting, nextest, Windows-target lint, generated assets/clients, docs, and maintenance suites pass.
