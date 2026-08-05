## Why

Pane and agent API responses expose `screen_detection_skipped`, but not the evidence source or reporting authority that produced the current state. This makes runtime observations ambiguous and encourages clients to treat installation or stored session identity as integration health.

## What Changes

- Add one neutral, typed state-evidence projection shared by `PaneInfo` and `AgentInfo`.
- Distinguish the evidence producing the effective state from an active mixed reporter that may yield to screen evidence.
- Preserve `screen_detection_skipped` as a compatibility field derived from the new projection.
- Keep session identity as a separate fact and expose descriptive integration observations without report-age health indicators.
- Generate matching schema, TypeScript, event, documentation, and focused test updates.

## Capabilities

### New Capabilities

- `agent-state-source`: Truthful state evidence and reporting-authority projection for runtime clients.

### Modified Capabilities

None.

## Impact

- Runtime state projection in `src/terminal/state.rs` and API projection in `src/app/creation.rs` and `src/app/agents.rs`.
- Public API models in `src/api/schema/agents.rs`, `src/api/schema/panes.rs`, and generated clients/schema.
- Integration observation aggregation in Settings after responsive integration management is available.
- No new dependency and no session persistence migration.
- Current protocol source is ahead of the latest released protocol; compare before changing wire fixtures and do not bump blindly.
- This follow-on supersedes the overlapping `agent-state-source` capability section in `surface-settings-and-integration-controls`; that change owns Settings presentation only.
- depends on: `surface-settings-and-integration-controls` — task 3.1 aggregates observations into the integration view model that change establishes.
- touches: `src/terminal/state.rs`, `src/app/creation.rs`, `src/app/agents.rs`, `src/app/state.rs`, `src/ui/settings.rs`, `src/api/schema.rs`, `src/api/schema/agents.rs`, `src/api/schema/panes.rs`, `src/api/schema/tests.rs`, `src/integration/types.rs`, `src/protocol/wire.rs`, `docs/next/api/shepherd-api.schema.json`, `docs/next/website/src/content/docs/socket-api.mdx`, `clients/ts/src/index.ts`, `clients/ts/test/client.test.ts`, `clients/ts/test/generate-types.test.ts`, `scripts/test_socket_api_reference_check.py`
