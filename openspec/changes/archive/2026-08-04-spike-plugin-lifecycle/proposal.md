## Why

Installed GitHub plugins record their resolved upstream provenance, but Shepherd previously gave users no way to learn that a tracked branch or tag had moved. The conservative drift-reporting implementation is now shipped in the working tree, including legacy-registry compatibility; the user has accepted the security decision to keep the feature manual and read-only rather than automatically update third-party code.

## What Changes

- Define the shipped `shepherd plugin outdated [--plugin ID] [--json]` contract for manual comparison of recorded GitHub provenance with one live upstream ref query.
- Preserve migration behavior for pre-provenance registry entries and local plugins.
- Record the accepted manual-only lifecycle policy: no automatic plugin update, background polling, code fetch, checkout, or execution.
- Archive the completed change normally after fresh focused and repository-wide validation.

## Capabilities

### New Capabilities

- `plugin-upstream-drift`: Manual, read-only plugin provenance comparison with explicit current, outdated, pinned, not-applicable, and unknown outcomes.

### Modified Capabilities

None.

## Impact

- CLI: `src/cli/spec.rs` and `src/cli/plugin.rs` expose the manual drift report and JSON output.
- Persistence: `src/persist/plugin_registry.rs` proves a source-less legacy entry still loads with local defaults; the stored schema does not require a migration.
- Network and trust: an eligible GitHub plugin causes one user-invoked `git ls-remote` query; the command never fetches or executes plugin code.
- Product boundary: no API/TUI surface, unattended polling, update command, marketplace trust expansion, or automatic code installation is introduced.
