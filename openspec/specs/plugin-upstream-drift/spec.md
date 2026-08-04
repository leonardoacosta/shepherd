# plugin-upstream-drift Specification

## Purpose
TBD - created by archiving change spike-plugin-lifecycle. Update Purpose after archive.
## Requirements
### Requirement: Plugin drift is reported manually without fetching code
Shepherd SHALL provide a user-invoked plugin drift report that compares recorded GitHub provenance with the current upstream ref without fetching, checking out, installing, or executing plugin code.

#### Scenario: Tracked upstream ref moved
- **WHEN** an installed GitHub plugin follows a branch, tag, or default `HEAD` whose current commit differs from its recorded resolved commit
- **THEN** `shepherd plugin outdated` reports the plugin as `outdated`
- **AND** the comparison uses one live ref query rather than the marketplace index

#### Scenario: Tracked upstream ref is unchanged
- **WHEN** the live ref resolves to the recorded commit
- **THEN** the command reports the plugin as `current`

#### Scenario: Installed plugin is local
- **WHEN** a plugin has local source provenance
- **THEN** the command reports `n/a`
- **AND** it performs no network query for that plugin

#### Scenario: Installed plugin is pinned to an exact commit
- **WHEN** a GitHub plugin's requested ref is an exact commit SHA
- **THEN** the command reports `pinned`
- **AND** it performs no network query for that plugin

#### Scenario: Provenance or remote resolution is insufficient
- **WHEN** the recorded commit is absent, the remote ref is missing or ambiguous, or the ref query fails
- **THEN** the command reports `unknown` with labeled detail for that plugin
- **AND** it continues reporting every other selected plugin

### Requirement: Legacy plugin registries remain loadable
Shepherd SHALL deserialize plugin registry entries written before source provenance existed by applying local source defaults.

#### Scenario: Registry entry has no source field
- **WHEN** Shepherd loads a valid legacy plugin registry entry without `source`
- **THEN** loading succeeds
- **AND** the entry behaves as a local plugin with no upstream comparison

### Requirement: Plugin drift does not authorize unattended updates
Shepherd SHALL keep upstream drift reporting manual and informational unless a separate approved proposal defines an update trust model.

#### Scenario: A plugin is outdated
- **WHEN** the manual report finds a newer upstream commit
- **THEN** Shepherd does not automatically fetch, install, or execute that commit
- **AND** the report itself remains a successful command outcome

#### Scenario: Shepherd is idle or starts the TUI
- **WHEN** the user has not invoked the drift command
- **THEN** Shepherd performs no background plugin-ref polling and exposes no implicit TUI update state
