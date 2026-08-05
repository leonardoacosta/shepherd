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

### Requirement: Shepherd resolves reader facts without the companion
Shepherd SHALL resolve credential, account, transcript-usage, pricing, and severity facts by
reading their sources itself. No such value SHALL depend on the companion binary being present.

#### Scenario: Companion binary is absent
- **WHEN** the companion binary is not installed and a configured row consumes an absorbed token
- **THEN** Shepherd resolves the value from the source directly
- **AND** the row renders it

#### Scenario: Render performs no resolution
- **WHEN** a frame renders an absorbed token
- **THEN** the value comes from the cached store
- **AND** no adapter work begins on the render path

#### Scenario: A source is unavailable
- **WHEN** an absorbed source is missing, unreadable, or unparseable
- **THEN** only its own tokens elide
- **AND** Shepherd surfaces no error dialog and writes no user-visible warning

### Requirement: Hook ingest stays in the companion
Shepherd SHALL NOT implement harness hook ingest. The companion SHALL remain the only component
that receives harness lifecycle events and writes the session snapshot store.

#### Scenario: A harness lifecycle event occurs
- **WHEN** the harness fires a session lifecycle event
- **THEN** the companion receives it and persists the resulting snapshot
- **AND** Shepherd does not participate in that path

#### Scenario: Shepherd reads what the companion wrote
- **WHEN** Shepherd resolves a fact whose source is the session snapshot store
- **THEN** Shepherd reads the persisted store as a provider source
- **AND** does not write to it

#### Scenario: Companion retains only ingest
- **WHEN** the companion is invoked with a subcommand whose responsibility moved to Shepherd
- **THEN** it exits with a usage error naming Shepherd as the owner of that fact

### Requirement: Credential material is read but never rendered
An adapter reading credential sources SHALL treat them as read-only and SHALL expose only derived
values. No credential secret SHALL reach a rendered token, a log, or an error message.

#### Scenario: A token backed by credential data renders
- **WHEN** a configured row consumes a token derived from credential sources
- **THEN** the rendered value is a derived count, quota, or time
- **AND** contains no credential secret

#### Scenario: A credential source fails to parse
- **WHEN** a credential source cannot be parsed
- **THEN** its tokens elide
- **AND** no diagnostic includes the source's contents

#### Scenario: Shepherd never writes a credential source
- **WHEN** Shepherd resolves a credential-derived fact
- **THEN** it opens the source read-only
- **AND** the source is unchanged afterwards
