## ADDED Requirements

### Requirement: Shepherd owns session and provider status
Shepherd SHALL resolve session and provider status itself, holding the cached value on a
pure-data object within application state and the refresh bookkeeping on the runtime. No
displayed value SHALL depend on an outside process reporting it in.

#### Scenario: A value is resolved without an outside reporter
- **WHEN** a configured row consumes a session or provider token and no outside process is running
- **THEN** shepherd resolves the value itself and the row renders it

#### Scenario: Render performs no resolution
- **WHEN** a frame renders a session or provider token
- **THEN** the value comes from the cached store
- **AND** no provider work begins on the render path

### Requirement: Session provider refresh is demand-driven
Shepherd SHALL attempt provider work only when a configured row consumes a token that needs it,
and SHALL resolve that demand per token rather than for the feature as a whole.

#### Scenario: No configured consumer attempts no work
- **WHEN** no configured row consumes a session or provider token
- **THEN** shepherd never starts a refresh
- **AND** attempts no provider work

#### Scenario: One configured token enables only its own adapter
- **WHEN** a configured row consumes one session token but not another
- **THEN** only the adapter backing the consumed token is invoked

#### Scenario: Refresh has its own cadence
- **WHEN** session provider status and Git status are both configured
- **THEN** session provider status refreshes on its own interval, independent of the Git status interval

### Requirement: Adapter failure degrades to an absent value
An adapter that is missing, exits non-zero, times out, or returns an unparseable body SHALL yield
no value for its own token, and SHALL NOT affect any other token, any other surface, or the Git
status path.

#### Scenario: Provider is unavailable
- **WHEN** an adapter's source cannot be reached or does not exist
- **THEN** its token resolves to no value
- **AND** shepherd surfaces no error dialog and writes no user-visible warning

#### Scenario: One adapter fails while another succeeds
- **WHEN** one adapter fails and another returns a value
- **THEN** only the failing adapter's token is absent
- **AND** the succeeding adapter's token renders normally

#### Scenario: Adapter does not terminate promptly
- **WHEN** an adapter does not return within its timeout
- **THEN** shepherd abandons that attempt and keeps the previously cached value, if any
- **AND** the refresh cadence is not delayed for other work
