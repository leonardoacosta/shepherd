## ADDED Requirements

### Requirement: A remote attach reports keybindings it did not send
Shepherd SHALL tell the user which of their local bindings were not sent when a client attaches
with local keybindings and its own configuration declares command bindings, and SHALL name the
side that owns command bindings for that session. Shepherd SHALL NOT present a local-keybinding
attach as carrying the user's complete local keybinding set when it does not.

#### Scenario: Local configuration declares command bindings
- **WHEN** a client with `[[keys.command]]` entries attaches using local keybindings
- **THEN** Shepherd reports that those command bindings were not sent
- **AND** states that the server's own command bindings apply instead

#### Scenario: Local configuration declares no command bindings
- **WHEN** a client with no `[[keys.command]]` entries attaches using local keybindings
- **THEN** Shepherd reports nothing about command bindings

#### Scenario: Server keybindings requested
- **WHEN** a client attaches with `--remote-keybindings server`
- **THEN** Shepherd reports nothing about unsent local command bindings

### Requirement: Remote keybinding guidance states both owners
Documentation SHALL state which side owns command bindings under each `--remote-keybindings` mode
and which side wins a chord collision, in every maintained locale describing that flag and in its
CLI help text. Documentation SHALL NOT describe the local mode only in terms of what the client
does not send.

#### Scenario: Reader consults the remote persistence documentation
- **WHEN** a user reads the documentation for a remote attach
- **THEN** it states that local command bindings are not sent
- **AND** that the server's own command bindings still apply
- **AND** that a server command wins a chord it shares with a client action binding

#### Scenario: Reader consults CLI help
- **WHEN** a user reads the help text for `--remote-keybindings`
- **THEN** it names what each mode takes from which side, including command bindings
