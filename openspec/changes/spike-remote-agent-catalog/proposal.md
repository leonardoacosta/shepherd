## Why

Shepherd's remote detection catalog can update only agents already known to the binary. `parse_catalog` resolves every remote id through `parse_agent_label` and discards unknown ids, while contributor guidance does not yet offer a focused, evidence-backed path for adding a curated agent. The approved direction is to make that contribution path explicit rather than de-enum the runtime and let unreviewed remote data introduce identities.

## What Changes

- Document the manifest shape, structural limits, local validation, and screen-evidence requirements for community agent-detection submissions.
- Add a focused pull-request checklist that requires captured classification evidence and explicit maintainer review of matcher meaning, not only automated syntax checks.
- Add the smallest durable fixture acceptance path needed to prove a proposed agent's representative states without matching incidental whole-screen text.
- Preserve the current curated, bundled, release-time identity model: unknown remote catalog ids remain rejected and the full string-id runtime architecture remains out of scope.

## Capabilities

### New Capabilities

- `community-agent-manifest-submissions`: A reviewable, evidence-backed community path for adding bundled agent detection manifests.

### Modified Capabilities

None.

## Impact

- Contribution surfaces: a new agent-detection submission guide and focused pull-request template under existing website and GitHub contribution paths.
- Detection tests: a minimal golden/captured-state acceptance harness may add representative screen fixtures and manifest test coverage.
- Runtime architecture: no change to `Agent`, `parse_agent_label`, pane lifecycle, wire schemas, catalog parsing, or remote update trust.
- Delivery: newly accepted agents still require a Shepherd release because identity remains curated and bundled.
