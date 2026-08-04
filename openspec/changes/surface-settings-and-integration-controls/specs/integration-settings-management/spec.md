## ADDED Requirements

### Requirement: Every integration target has a reachable status row
The Integrations section SHALL expose every registered target by stable `IntegrationTarget` identity with its support/availability, install state, installed path, current version, and expected version when those facts exist.

#### Scenario: Current installation
- **WHEN** a target is installed at the expected version
- **THEN** its row identifies it as current and its detail exposes the installed version and path

#### Scenario: Outdated installation
- **WHEN** a target's installed version differs from the expected version
- **THEN** its row identifies an update action and detail exposes both versions

#### Scenario: Missing installation
- **WHEN** a supported and available target is not installed
- **THEN** its row identifies an install action without fabricating path or version values

#### Scenario: Unsupported or unavailable target
- **WHEN** a registered target cannot be managed on the current platform/build
- **THEN** it remains reachable with an explanatory disabled state and cannot invoke install or uninstall

#### Scenario: Small viewport
- **WHEN** Integrations renders at 40x20 with every registered target
- **THEN** navigation and scrolling can reach each target and the selected identity remains visible

### Requirement: Per-target operations are explicit and safe
The Integrations section SHALL invoke existing neutral operations for the selected target, SHALL serialize integration mutations within the server session, and SHALL require confirmation before uninstall.

#### Scenario: Install selected target
- **WHEN** the user confirms install on a missing available target
- **THEN** only that target is passed to the neutral install operation and status refreshes after completion

#### Scenario: Update selected target
- **WHEN** the user confirms update on an outdated target
- **THEN** the target's install operation runs and refreshed status reflects the resulting version

#### Scenario: Confirm uninstall
- **WHEN** the user requests uninstall of an installed target
- **THEN** a confirmation names the target and known path and no uninstall occurs before explicit acceptance

#### Scenario: Cancel uninstall
- **WHEN** the uninstall confirmation is cancelled
- **THEN** no integration operation or filesystem mutation occurs

#### Scenario: Duplicate operation
- **WHEN** any target operation or bulk sequence is already running
- **THEN** every integration mutation action is disabled or coalesced while target navigation and existing result detail remain available

#### Scenario: Keyboard and mouse target parity
- **WHEN** keyboard activation and mouse activation address the same visible target row/action
- **THEN** both dispatch the same `IntegrationTarget` identity and confirmation behavior

### Requirement: Bulk installation remains secondary and bounded
The Integrations section SHALL retain a secondary bulk action for recommended missing/outdated targets and SHALL exclude targets that are unsupported or unavailable.

#### Scenario: Mixed bulk eligibility
- **WHEN** recommendations include current, missing, outdated, unsupported, and unavailable targets
- **THEN** bulk install serially attempts only eligible missing/outdated targets in registry order and records an individual result for each attempt

#### Scenario: No eligible target
- **WHEN** no target needs an eligible install/update
- **THEN** the bulk action is absent or disabled and does not invoke an empty mutation

### Requirement: Operation output remains completely reachable
Integration operation results SHALL be stored by target and operation with complete message lines, shown through a bounded summary and scrollable detail without discarding late failures.

#### Scenario: More results than fit
- **WHEN** a bulk or repeated operation produces more result lines than the content viewport
- **THEN** every result and message remains reachable through selection/scrolling

#### Scenario: Late partial failure
- **WHEN** several targets succeed and a later target fails
- **THEN** the failing target is visible in the summary and its complete diagnostic is reachable at 40x20

#### Scenario: Status refresh differs from command output
- **WHEN** an operation reports success but refreshed filesystem/version status does not become current
- **THEN** Settings preserves the command result and separately shows the refreshed status without claiming success as health

### Requirement: Runtime observations remain descriptive
The Integrations section SHALL aggregate current state-source and session-identity facts by integration source without exposing session identifiers or assigning health.

#### Scenario: Full lifecycle reports exist
- **WHEN** an integration is the exclusive reported state source for two current panes
- **THEN** its detail describes state reporting in two panes and does not call installation healthy

#### Scenario: Session identity only
- **WHEN** an integration supplies stored session identity but no current state report
- **THEN** its detail distinguishes session identity availability from lifecycle reporting

#### Scenario: No observation
- **WHEN** an integration is installed but no current pane supplies its state or session source
- **THEN** Settings says it is not currently observed and does not label it broken

#### Scenario: Unknown reporting source
- **WHEN** a pane reports a source id that does not exactly map to a registered `IntegrationTarget`
- **THEN** Integration Settings does not guess a target or include that pane in any target's observation count
