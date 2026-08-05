# agent-state-source Specification

## Purpose
TBD - created by archiving change expose-agent-state-source. Update Purpose after archive.
## Requirements
### Requirement: API responses expose effective state evidence

The server SHALL expose a typed state-evidence projection on `PaneInfo` and `AgentInfo` whenever a pane has current agent evidence. The projection SHALL identify whether the effective state came from screen detection or a reported source and, for reported state, whether the source has exclusive lifecycle authority or mixed authority.

#### Scenario: Screen-derived state

- **WHEN** a pane is classified by its screen manifest and has no effective lifecycle report
- **THEN** `PaneInfo` and `AgentInfo` report screen evidence and agree on the same value

#### Scenario: Exclusive lifecycle report

- **WHEN** a live canonical full-lifecycle integration owns the pane's state
- **THEN** both API models report the canonical source with exclusive lifecycle authority and compatibility `screen_detection_skipped` remains true

#### Scenario: Mixed report overridden by screen evidence

- **WHEN** a mixed reporter is active but a newer visible screen blocker determines the effective state
- **THEN** the projection identifies screen evidence as effective and retains the reporter as mixed observation without claiming the reporter authored that state

### Requirement: Projection clears stale authority

The server SHALL remove reporter observations when authority is cleared, the owning process exits, or the report is otherwise no longer effective. A persisted session reference alone SHALL NOT create a current reporter observation.

#### Scenario: Cleared report falls back

- **WHEN** a hook authority is cleared and current screen evidence exists
- **THEN** the projection reports screen evidence and no active reporter

#### Scenario: Session identity without live report

- **WHEN** a pane has a persisted or newly reported session identity but no current state authority
- **THEN** session identity remains separate and the state-evidence projection does not claim active reporting

### Requirement: Integration observations remain descriptive

Clients SHALL aggregate only exact canonical source matches for named integrations. They SHALL distinguish current state reporters from session-identity observations and SHALL describe zero current observations without presenting installation, report age, or absence as health failure.

#### Scenario: Current canonical observation

- **WHEN** two panes currently report through the exact source owned by an installed integration
- **THEN** the client may display that the integration is reporting state in two panes

#### Scenario: Installed but not observed

- **WHEN** an integration is installed but no current pane reports through its canonical source
- **THEN** the client describes it as installed and not currently observed

#### Scenario: Unknown custom source

- **WHEN** a pane reports through an unregistered or mismatched custom source
- **THEN** the client does not attribute that report to a named integration

