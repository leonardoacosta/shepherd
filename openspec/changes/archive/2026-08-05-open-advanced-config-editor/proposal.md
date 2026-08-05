## Why

Shepherd exposes only a curated subset of its 153 configuration keys through Settings, while advanced layouts, keybindings, integrations, and platform options remain TOML-only. Users need a safe path to the real server configuration without a duplicated form or an embedded editor that is weaker than their existing editor.

## What Changes

- Add a neutral `server.config.edit` operation that opens the resolved server configuration in an editor pane.
- Preserve the actual file and its bytes; never treat it as a temporary overlay file.
- Validate changed content after editor exit, reload valid edits, retain the last valid runtime configuration for invalid edits, and return actionable diagnostics.
- Propagate editor exit status and publish a completion event/result for remote-capable clients.
- Keep one active config editor per server session and return its existing pane on duplicate requests.
- Use platform-safe editor argv construction with `VISUAL`/`EDITOR` precedence on Unix and existing Windows command-line parsing.

## Capabilities

### New Capabilities

- `advanced-config-editing`: Safe server-owned editor access and validated reload lifecycle.

### Modified Capabilities

None.

## Impact

- Server/API schema, internal pane-exit events, editor process lifecycle, config reload, and platform argv helpers.
- TUI Settings/global action presentation and next API/configuration documentation.
- No new dependency and no config-file migration.
- The operation is server-owned and does not accept a client-local path or require TUI-private socket semantics.
- This follow-on supersedes the overlapping `advanced-config-editing` capability section in `surface-settings-and-integration-controls`; that change owns curated Settings rows only.
- depends on: `surface-settings-and-integration-controls` — task 3.1 adds the advanced action row to the responsive Settings foundation that change establishes.
- touches: `src/api/schema.rs`, `src/api/schema/server.rs`, `src/api/schema/events.rs`, `src/api/schema/response.rs`, `src/api/schema/tests.rs`, `src/app/api.rs`, `src/app/mod.rs`, `src/app/state.rs`, `src/app/config_io.rs`, `src/app/runtime_mutations.rs`, `src/app/input/navigate.rs`, `src/app/input/settings.rs`, `src/events.rs`, `src/ui/settings.rs`, `src/config/io.rs`, `src/platform/mod.rs`, `src/platform/linux.rs`, `src/platform/macos.rs`, `src/platform/windows.rs`, `src/platform/fallback.rs`, `src/server/headless.rs`, `src/protocol/wire.rs`, `docs/next/api/shepherd-api.schema.json`, `docs/next/website/src/content/docs/socket-api.mdx`, `docs/next/website/src/content/docs/configuration.mdx`, `clients/ts/src/index.ts`, `clients/ts/test/client.test.ts`, `clients/ts/test/generate-types.test.ts`, `scripts/test_socket_api_reference_check.py`
