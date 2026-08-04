## ADDED Requirements

### Requirement: Public agent state identifies its evidence source
`PaneInfo` and `AgentInfo` SHALL expose an optional typed state-source value that is either screen-derived or reported with a neutral source identifier and `exclusive_lifecycle` or `mixed` authority.

#### Scenario: Screen-manifest state
- **WHEN** current agent state is derived from screen detection without active report authority
- **THEN** both public models expose the `screen` state source

#### Scenario: Exclusive lifecycle report
- **WHEN** current hook authority supplies full lifecycle state and suppresses screen detection
- **THEN** both public models expose the reporting source with `exclusive_lifecycle` authority

#### Scenario: Mixed reported evidence
- **WHEN** a current report supplies state but screen evidence may still participate
- **THEN** both public models expose the reporting source with `mixed` authority

#### Scenario: No current agent state
- **WHEN** a pane has neither current reported nor screen-derived agent state
- **THEN** its state-source field is absent

### Requirement: State-source projection has one terminal authority
The server SHALL derive PaneInfo and AgentInfo state-source values through the same terminal projection helper so the two models cannot disagree for the same terminal state.

#### Scenario: Same terminal through pane and agent APIs
- **WHEN** one terminal is returned through pane and agent list/get paths
- **THEN** its state-source values are equal

#### Scenario: Authority clears or expires
- **WHEN** reported authority is cleared or no longer active
- **THEN** both models atomically fall back to `screen` when screen evidence exists or to absent otherwise

#### Scenario: Restore contains session identity
- **WHEN** persisted session identity is restored without a current lifecycle report
- **THEN** session identity remains separate and state source is not reported merely because the session reference exists

### Requirement: Compatibility boolean derives from typed authority
The existing `screen_detection_skipped` field SHALL remain compatible and SHALL be true exactly when the typed state source is an exclusive lifecycle report.

#### Scenario: Exclusive report
- **WHEN** state source is `reported` with `exclusive_lifecycle`
- **THEN** `screen_detection_skipped` is true

#### Scenario: Screen or mixed report
- **WHEN** state source is `screen`, `reported` with `mixed`, or absent
- **THEN** `screen_detection_skipped` is false

### Requirement: State source is not a heartbeat
The public state-source contract SHALL NOT assign health, stale thresholds, or traffic-light meaning to report age.

#### Scenario: Reporter is idle
- **WHEN** an accepted reporter has not emitted a recent transition but its authority remains current under terminal rules
- **THEN** the API reports its source/authority without a healthy, unhealthy, or stale classification
