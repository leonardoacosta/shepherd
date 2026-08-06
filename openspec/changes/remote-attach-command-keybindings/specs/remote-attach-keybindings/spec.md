## ADDED Requirements

### Requirement: A local-keybinding attach adopts client actions and server commands
When a client attaches with `--remote-keybindings local`, Shepherd SHALL resolve the live keybind
set from two owners: the prefix and built-in action bindings from the client's transmitted
profile, and the custom command bindings from the server's own configuration. A local-keybinding
attach SHALL NOT leave the command bindings empty when the server's configuration declares them.

#### Scenario: Server command bindings apply under a local attach
- **WHEN** a client attaches with local keybindings and the server configuration declares a
  `[[keys.command]]` entry
- **THEN** the resolved keybind set contains that command binding
- **AND** pressing its chord runs the command on the server host

#### Scenario: Client action bindings still apply
- **WHEN** a client attaches with local keybindings and its profile customizes a built-in action
- **THEN** the resolved keybind set uses the client's binding for that action
- **AND** uses the client's prefix

#### Scenario: Server declares no command bindings
- **WHEN** a client attaches with local keybindings and the server configuration declares no
  `[[keys.command]]` entries
- **THEN** the resolved keybind set contains no command bindings
- **AND** the attach succeeds without a diagnostic

### Requirement: Client-supplied command bindings are refused
Shepherd SHALL NOT execute a command named by an attaching client's keybindings payload. The
server SHALL discard command entries carried in a received keybindings profile before resolving
the live keybind set, regardless of how that payload was produced.

#### Scenario: Payload carrying command entries
- **WHEN** a received keybindings profile contains `[[keys.command]]` entries
- **THEN** Shepherd discards them before resolving the keybind set
- **AND** no chord in the resulting set runs a command named by that payload

#### Scenario: Discarded client commands do not suppress server commands
- **WHEN** a received keybindings profile contains command entries and the server configuration
  also declares command bindings
- **THEN** Shepherd discards the client's entries
- **AND** the server's own command bindings remain in the resolved set

### Requirement: Server command bindings win a chord collision
Shepherd SHALL bind a contested chord to the server command when a server command binding and a
client action binding resolve to the same chord under a local-keybinding attach. This SHALL match
the precedence a local session on that host applies when a `[[keys.command]]` entry and a built-in
action share a chord.

#### Scenario: Command displaces a built-in action on the same chord
- **WHEN** the server binds a command to a chord that the client profile binds to a built-in
  action
- **THEN** pressing the chord runs the server command
- **AND** does not invoke the built-in action

#### Scenario: Non-colliding bindings both apply
- **WHEN** the server's command chords and the client's action chords do not overlap
- **THEN** every server command chord runs its command
- **AND** every client action chord invokes its action

### Requirement: A server-keybinding attach is unchanged
When a client attaches with `--remote-keybindings server`, Shepherd SHALL resolve the live keybind
set entirely from the server's own configuration, including its prefix, action bindings, and
command bindings.

#### Scenario: Server keybindings requested
- **WHEN** a client attaches with `--remote-keybindings server`
- **THEN** the resolved keybind set is the server's own configured set
- **AND** the client's local configuration does not contribute to it
