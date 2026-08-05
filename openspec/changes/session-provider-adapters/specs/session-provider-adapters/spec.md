## ADDED Requirements

### Requirement: Each source has its own failure envelope
Shepherd SHALL invoke one adapter per session-provider source and SHALL scope each adapter's
failure to the tokens that source backs. No source's unavailability SHALL blank a token backed by
another source.

#### Scenario: One source fails while another succeeds
- **WHEN** one adapter's source is unavailable and another adapter returns a value
- **THEN** only the unavailable source's tokens elide
- **AND** the available source's tokens render their current values

#### Scenario: Every source is unavailable
- **WHEN** no adapter's source can be reached
- **THEN** every session token elides
- **AND** Shepherd surfaces no error dialog and writes no user-visible warning
- **AND** every token backed by another feature in the same row renders unchanged

#### Scenario: A source recovers
- **WHEN** an adapter whose source was unavailable returns a value on a later refresh
- **THEN** its tokens render again
- **AND** no other source's tokens change as a result

### Requirement: Demand is resolved per token
Shepherd SHALL run an adapter only when a configured row consumes a token that adapter backs.

#### Scenario: One configured token enables one adapter
- **WHEN** a configured row consumes a token backed by exactly one source
- **THEN** only that source's adapter runs
- **AND** no other adapter is invoked

#### Scenario: No configured consumer runs no adapter
- **WHEN** no configured row consumes any session token
- **THEN** Shepherd invokes no adapter and starts no refresh

#### Scenario: Two tokens sharing one source run one adapter
- **WHEN** a configured row consumes two tokens backed by the same source
- **THEN** that source's adapter runs once per refresh

### Requirement: Adapter mechanism is chosen per source
Each adapter SHALL read its source by whichever mechanism that source provides, and Shepherd
SHALL NOT impose one mechanism across all adapters.

#### Scenario: A source that is a program
- **WHEN** an adapter's source is an executable
- **THEN** the adapter invokes it and treats absence, non-zero exit, timeout, and unparseable
  output as no value

#### Scenario: A source that is a file
- **WHEN** an adapter's source is a file or directory
- **THEN** the adapter reads it and treats absence, unreadable content, and unparseable content
  as no value

#### Scenario: A partially readable file source
- **WHEN** a file source contains both parseable and unparseable entries
- **THEN** the adapter resolves from the parseable entries
- **AND** does not fail the whole token because one entry is malformed
