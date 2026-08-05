# advanced-config-editing Specification

## Purpose
TBD - created by archiving change open-advanced-config-editor. Update Purpose after archive.
## Requirements
### Requirement: Server owns the edited configuration path

The server SHALL expose a `server.config.edit` operation that resolves the active server configuration path itself. The operation SHALL reject client-provided paths and SHALL create a missing parent directory without replacing an absent configuration file.

#### Scenario: Resolved path is opened

- **WHEN** a client requests config editing and an editor pane can be hosted
- **THEN** the server opens its resolved config path and returns the pane identity

#### Scenario: Client attempts path injection

- **WHEN** a request includes a client-local or alternate path
- **THEN** the server rejects the request without opening or modifying that path

#### Scenario: TUI presentation

- **WHEN** the TUI receives a newly started editor pane
- **THEN** it may present that pane as an overlay without changing the API's neutral pane/runtime semantics

#### Scenario: No existing config file

- **WHEN** the resolved config path does not yet exist and the editor exits without saving
- **THEN** the outcome is unchanged and Shepherd does not synthesize or delete a file

### Requirement: Editor operations are single-flight

The server SHALL allow at most one Shepherd-owned config editor per session. A duplicate request SHALL return the existing pane identity and SHALL NOT start a second editor. The operation SHALL continue independently of the initiating client's connection.

#### Scenario: Duplicate request

- **WHEN** a config editor is already active
- **THEN** the request returns `already_open` with the existing pane identity

#### Scenario: No hosting surface

- **WHEN** the server cannot host an editor pane
- **THEN** the request returns a surface-unavailable error and leaves the config unchanged

#### Scenario: Initiating client disconnects

- **WHEN** the initiating client disconnects after the editor pane starts
- **THEN** the server keeps the operation alive and performs exit validation normally

#### Scenario: Ordinary navigation and restore

- **WHEN** an editor auxiliary pane is live or a session snapshot is written
- **THEN** it is excluded from ordinary tab navigation and disk restore while retaining stable runtime identity until exit

### Requirement: Editor argv is platform-safe

The server SHALL pass the resolved config path as one positional editor argument. Unix SHALL use `VISUAL`, then `EDITOR`, then `vi`; Windows SHALL use safe command-line parsing and fall back to Notepad. The config path SHALL NOT be registered for temporary overlay deletion.

#### Scenario: Path contains shell-significant characters

- **WHEN** the resolved path contains spaces or shell-significant characters
- **THEN** the editor receives it as one argument without shell interpolation corruption

#### Scenario: No editor configured

- **WHEN** neither editor environment variable is usable
- **THEN** the platform fallback editor is launched with the config path

#### Scenario: Unix editor contains arguments

- **WHEN** `$VISUAL` or `$EDITOR` contains a command plus arguments on Linux or macOS
- **THEN** those command semantics are preserved and a config path containing spaces or shell metacharacters is received as one path argument

#### Scenario: Windows editor contains arguments

- **WHEN** the configured Windows editor contains quoted executable/arguments
- **THEN** existing safe argv parsing preserves boundaries and appends the config path as one argument

#### Scenario: Editor pane exits

- **WHEN** the auxiliary editor pane exits
- **THEN** cleanup does not remove the real config file

### Requirement: Changed content is validated before reload

The server SHALL compare final file bytes with the pre-edit snapshot. Unchanged content SHALL produce an unchanged result. Changed valid TOML SHALL use the existing reload pipeline. Changed invalid or unreadable content SHALL remain on disk while the last valid runtime configuration remains active.

#### Scenario: Valid edit

- **WHEN** the editor changes the file to valid TOML and exits
- **THEN** the server reloads the valid configuration and reports a reloaded outcome

#### Scenario: Invalid edit

- **WHEN** the editor leaves invalid TOML
- **THEN** the server preserves the invalid bytes, keeps the last valid runtime configuration, and reports diagnostics with the config path

#### Scenario: Unchanged edit

- **WHEN** the editor exits successfully without changing the resolved file
- **THEN** the completion outcome is `unchanged` and no unnecessary configuration mutation occurs

#### Scenario: Nonzero editor exit with valid bytes

- **WHEN** the editor exits nonzero but leaves changed valid TOML
- **THEN** the server reloads the valid bytes and includes the editor failure as diagnostic context

#### Scenario: Editor process failure

- **WHEN** the editor process exits unsuccessfully without changing the resolved file
- **THEN** the completion outcome is `editor_failed`, current runtime configuration remains active, and diagnostics remain reachable

#### Scenario: Existing config is removed

- **WHEN** the config existed before editing and is absent when the editor exits
- **THEN** Shepherd treats the deliberate removal as a valid change, reloads defaults, and reports the removal diagnostically without deleting any file itself

#### Scenario: Changed config is unreadable

- **WHEN** final changed config bytes cannot be read or validated
- **THEN** Shepherd leaves the filesystem untouched, retains last valid runtime state, and reports `invalid` with the I/O diagnostic

#### Scenario: External edit during operation

- **WHEN** another process changes the config while the editor operation is live
- **THEN** the final bytes at editor exit are validated as authoritative and Shepherd does not claim cross-process locking

### Requirement: Completion is observable

The server SHALL publish a config-editor completion result containing pane identity, outcome, and diagnostics. Remote-capable clients SHALL be able to present the result and immediately request correction by reopening the same server-owned file.

#### Scenario: Completion event

- **WHEN** the editor pane exits
- **THEN** subscribers receive a completion event with a stable outcome and actionable diagnostics

#### Scenario: Every terminal outcome emits once

- **WHEN** an edit operation reaches `reloaded`, `unchanged`, `invalid`, or `editor_failed`
- **THEN** subscribers receive exactly one completion event for that pane and outcome

#### Scenario: Protocol compatibility accounting

- **WHEN** the method, event, or typed source fields change the wire contract
- **THEN** implementation compares the source protocol version to the latest release and bumps/updates fixtures only if the source is not already ahead

