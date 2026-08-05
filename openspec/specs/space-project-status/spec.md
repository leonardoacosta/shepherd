# space-project-status Specification

## Purpose
TBD - created by archiving change space-project-status. Update Purpose after archive.
## Requirements
### Requirement: Space rows expose project work-queue tokens
The space token vocabulary SHALL include a `proposals` token and a `beads` token. Each SHALL
resolve to a compact count string derived from the project state of the checkout that the space
resolves to, and SHALL follow the same styling, elision, and row-omission rules as every other
space token.

#### Scenario: Both tokens resolve
- **WHEN** a configured space row contains `proposals` and `beads`, and the space's checkout has
  both open change proposals and an issue database
- **THEN** Shepherd renders both counts in the configured order
- **AND** applies any configured foreground, bold, or dim style to each

#### Scenario: One provider is unavailable
- **WHEN** a configured space row contains both tokens and the space's checkout has change
  proposals but no issue database
- **THEN** Shepherd renders the proposals count
- **AND** elides the beads token and its separator
- **AND** leaves every other token in the row unchanged

#### Scenario: Neither provider is available
- **WHEN** a configured space row contains only the two project tokens and the space's checkout
  has neither change proposals nor an issue database
- **THEN** Shepherd omits the row entirely, as it does for any fully unresolved row

#### Scenario: Unknown token name is rejected
- **WHEN** a configuration declares a space token name that is not in the documented vocabulary
- **THEN** Shepherd reports a configuration error and does not start with that row

### Requirement: Project status refresh is demand-driven
Shepherd SHALL refresh project status only when a configured space row consumes a token that
needs it, and SHALL resolve that demand per token rather than for the feature as a whole.

#### Scenario: No configured consumer starts no refresh
- **WHEN** no configured space row contains `proposals` or `beads`
- **THEN** Shepherd never starts a project-status refresh
- **AND** spawns no provider process

#### Scenario: One configured token enables only its own provider
- **WHEN** a configured space row contains `proposals` but not `beads`
- **THEN** Shepherd refreshes proposal counts
- **AND** does not spawn the issue-database provider

#### Scenario: Refresh runs off the render path
- **WHEN** a project-status refresh is due
- **THEN** Shepherd performs the provider work without blocking a frame
- **AND** renders the previously cached values until the refresh completes

#### Scenario: Refresh has its own cadence
- **WHEN** project status and Git status are both configured
- **THEN** project status refreshes on its own interval, independent of the Git status interval

### Requirement: Project status is resolved once per checkout
Shepherd SHALL key project status by the checkout a space resolves to, and SHALL collapse spaces
that resolve to the same checkout into a single refresh.

#### Scenario: Several spaces share one checkout
- **WHEN** three spaces resolve to the same checkout and a refresh is due
- **THEN** Shepherd performs one refresh for that checkout
- **AND** all three spaces render the same resolved counts

#### Scenario: Linked worktrees are distinct checkouts
- **WHEN** two spaces resolve to two linked worktrees of one repository
- **THEN** Shepherd refreshes each checkout separately
- **AND** each space renders the counts of its own checkout

### Requirement: Provider failure degrades to an absent value
A provider that is missing, exits non-zero, times out, or returns an unparseable body SHALL
yield no value for its own token, and SHALL NOT affect any other token, any other space, or the
Git status path.

#### Scenario: Provider binary is not installed
- **WHEN** a provider's executable is not on `PATH`
- **THEN** its token resolves to no value
- **AND** Shepherd surfaces no error dialog and writes no user-visible warning

#### Scenario: Provider returns an unparseable body
- **WHEN** a provider exits zero but returns a body that does not match its expected shape
- **THEN** its token resolves to no value
- **AND** the other provider's token is unaffected

#### Scenario: Provider does not terminate promptly
- **WHEN** a provider does not return within its timeout
- **THEN** Shepherd abandons that probe and keeps the previously cached value, if any
- **AND** the refresh cadence is not delayed for other checkouts

