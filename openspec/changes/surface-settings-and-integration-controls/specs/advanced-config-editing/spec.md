## ADDED Requirements

### Requirement: Advanced editing targets the resolved server configuration
The server SHALL expose neutral method `server.config.edit` that resolves its own active configuration path, starts an editor in a server-owned auxiliary pane, and returns `ConfigEditStarted` with pane information and whether that operation already existed.

#### Scenario: Remote-capable client opens editor
- **WHEN** any authorized client invokes `server.config.edit`
- **THEN** the server uses its own resolved config path and does not accept or require a client-local path

#### Scenario: TUI presentation
- **WHEN** the TUI receives a newly started editor pane
- **THEN** it may present that pane as an overlay without changing the API's neutral pane/runtime semantics

#### Scenario: Editor unavailable
- **WHEN** no valid editor command can be constructed or the pane cannot start
- **THEN** the method returns `config_editor_unavailable` or a specific pane-start error and does not create, delete, or rewrite the config file

#### Scenario: No auxiliary pane surface
- **WHEN** the server has no session surface capable of hosting the auxiliary editor pane
- **THEN** the method returns `config_editor_surface_unavailable` without accepting a workspace identity or mutating configuration

#### Scenario: No existing config file
- **WHEN** the resolved config path does not yet exist and the editor exits without saving
- **THEN** the outcome is unchanged and Shepherd does not synthesize or delete a file

### Requirement: Editor argv preserves command and path boundaries
Platform editor construction SHALL preserve configured editor arguments while passing the resolved config path as a distinct safely quoted argument, and SHALL never register that path for temporary-file cleanup.

#### Scenario: Unix editor contains arguments
- **WHEN** `$VISUAL` or `$EDITOR` contains a command plus arguments on Linux or macOS
- **THEN** those command semantics are preserved and a config path containing spaces or shell metacharacters is received as one path argument

#### Scenario: Windows editor contains arguments
- **WHEN** the configured Windows editor contains quoted executable/arguments
- **THEN** existing safe argv parsing preserves boundaries and appends the config path as one argument

#### Scenario: Environment is unset
- **WHEN** no configured editor exists
- **THEN** Linux/macOS use `vi` and Windows uses Notepad through platform-isolated helpers

#### Scenario: Editor pane exits
- **WHEN** the auxiliary editor pane exits
- **THEN** cleanup does not remove the real config file

### Requirement: Config edit operations are single-flight and server-owned
Only one active config editor operation SHALL exist per server session, and it SHALL continue independently of the initiating client's connection.

#### Scenario: Duplicate request
- **WHEN** `server.config.edit` is called while its editor pane is live
- **THEN** the response returns that pane with `already_open: true` and does not launch a second editor

#### Scenario: Initiating client disconnects
- **WHEN** the initiating client disconnects after the editor pane starts
- **THEN** the server keeps the operation alive and performs exit validation normally

#### Scenario: Ordinary navigation and restore
- **WHEN** an editor auxiliary pane is live or a session snapshot is written
- **THEN** it is excluded from ordinary tab navigation and disk restore while retaining stable runtime identity until exit

### Requirement: Exit validation protects the running configuration
After the editor exits, the server SHALL validate the resolved file and reload only valid content, while retaining invalid user bytes and the last valid runtime configuration.

#### Scenario: Valid edit
- **WHEN** the editor exits after saving valid TOML
- **THEN** the normal reload pipeline applies it and the completion outcome is `reloaded`

#### Scenario: Invalid edit
- **WHEN** the editor exits after saving invalid TOML
- **THEN** the invalid file remains available for correction, last valid runtime values remain active, and diagnostics identify path plus line/column when available

#### Scenario: Unchanged edit
- **WHEN** the editor exits successfully without changing the resolved file
- **THEN** the completion outcome is `unchanged` and no unnecessary configuration mutation occurs

#### Scenario: Editor process failure
- **WHEN** the editor process exits unsuccessfully without changing the resolved file
- **THEN** the completion outcome is `editor_failed`, current runtime configuration remains active, and diagnostics remain reachable

#### Scenario: Nonzero exit after valid save
- **WHEN** the editor exits unsuccessfully after changing the file to valid TOML
- **THEN** Shepherd reloads the valid final bytes, reports `reloaded`, and retains the process failure as a diagnostic

#### Scenario: Existing config is removed
- **WHEN** the config existed before editing and is absent when the editor exits
- **THEN** Shepherd treats the deliberate removal as a valid change, reloads defaults, and reports the removal diagnostically without deleting any file itself

#### Scenario: Changed config is unreadable
- **WHEN** final changed config bytes cannot be read or validated
- **THEN** Shepherd leaves the filesystem untouched, retains last valid runtime state, and reports `invalid` with the I/O diagnostic

#### Scenario: External edit during operation
- **WHEN** another process changes the config while the editor operation is live
- **THEN** the final bytes at editor exit are validated as authoritative and Shepherd does not claim cross-process locking

### Requirement: Config edit completion is observable
The server SHALL publish `server.config_edit_finished` with pane identity, outcome, and diagnostics through the public event/subscription contract, and the TUI SHALL retain bounded feedback with an immediate reopen path.

#### Scenario: Valid completion event
- **WHEN** a config editor operation reloads successfully
- **THEN** subscribers receive one completion event for its pane with `reloaded` and Settings reflects success

#### Scenario: Invalid completion event
- **WHEN** validation fails
- **THEN** subscribers receive one `invalid` event with diagnostics and Settings offers reopening the same resolved file

#### Scenario: Every terminal outcome emits once
- **WHEN** an edit operation reaches `reloaded`, `unchanged`, `invalid`, or `editor_failed`
- **THEN** subscribers receive exactly one completion event for that pane and outcome

#### Scenario: Protocol compatibility accounting
- **WHEN** the method, event, or typed source fields change the wire contract
- **THEN** implementation compares the source protocol version to the latest release and bumps/updates fixtures only if the source is not already ahead
