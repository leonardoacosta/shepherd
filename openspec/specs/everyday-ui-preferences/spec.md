# everyday-ui-preferences Specification

## Purpose
TBD - created by archiving change surface-settings-and-integration-controls. Update Purpose after archive.
## Requirements
### Requirement: Settings exposes the selected everyday preferences
Settings SHALL expose pane borders, pane gaps, hide-single-tab-bar, and Agent sort under Display, and close confirmation, tab/workspace naming prompts, copy-on-select, and mouse scroll lines under Behavior.

#### Scenario: Display preferences
- **WHEN** the Display section renders
- **THEN** every selected visual preference is reachable by typed row identity alongside existing chrome controls

#### Scenario: Behavior preferences
- **WHEN** the Behavior section renders
- **THEN** every selected interaction preference is reachable without exposing excluded advanced/startup-only controls

#### Scenario: Advanced escape hatch
- **WHEN** the Behavior section renders
- **THEN** its final action row is `edit config.toml` and invokes the neutral server edit operation rather than representing advanced keys as native controls

#### Scenario: Dynamic row capacity
- **WHEN** the selected preference rows exceed the viewport at 40x20
- **THEN** stable selection and scrolling make every row reachable

### Requirement: Native preference changes preserve configuration text
Each native preference action SHALL update only its corresponding TOML key through comment-preserving persistence and SHALL reload the effective configuration after a successful write.

#### Scenario: Existing comments and unrelated keys
- **WHEN** a preference is changed in a config containing comments, custom ordering, and unrelated keys
- **THEN** only the selected key's semantic value changes and surrounding content is preserved

#### Scenario: Missing key
- **WHEN** the selected key is absent
- **THEN** the writer inserts the narrowest valid key without rewriting the whole document

#### Scenario: Malformed config
- **WHEN** existing TOML cannot be safely patched or reloaded
- **THEN** the runtime retains its last valid value and Settings shows actionable bounded feedback

#### Scenario: Failed write
- **WHEN** persistence fails
- **THEN** Settings does not pretend the value was saved or leave client state diverged from the effective config

### Requirement: Preference effect timing is truthful
Settings SHALL apply supported values live and SHALL explicitly label any value that can only take effect on next launch.

#### Scenario: Live preference
- **WHEN** a live-supported preference is saved
- **THEN** its existing consumers observe the reloaded value without restarting Shepherd

#### Scenario: Next-launch-only discovery
- **WHEN** implementation proves a selected value cannot safely apply live
- **THEN** its row and completion feedback say `next launch`, and tests assert that current runtime behavior is unchanged until restart

### Requirement: Agent sort has one persisted source
The contextual agent-sort toggle and the Settings control SHALL read and write the same
`ui.agent_panel_sort` key and SHALL use clarified contextual labels. The contextual toggle SHALL
live on the merged space-and-agent list rather than on a separate agent panel, and the sort SHALL
order agent rows within each space.

#### Scenario: Sidebar toggles sort
- **WHEN** the contextual control changes from grouped to priority
- **THEN** the header reads `sort: priority`, the shared key persists, and Settings reflects priority

#### Scenario: Settings changes sort
- **WHEN** Settings changes Agent sort
- **THEN** the merged list's ordering and label update from the same live configuration without a second state source

#### Scenario: Sort applies within every space
- **WHEN** the sort changes and several spaces are running agents
- **THEN** each space's agent rows reorder consistently beneath their own space row
- **AND** no agent row moves to a different space

### Requirement: Numeric preferences obey config bounds
Settings SHALL constrain mouse scroll lines and pane gap controls to the same accepted bounds and normalization rules as the configuration parser.

#### Scenario: Decrement at minimum
- **WHEN** a numeric preference is already at its minimum and decrement is requested
- **THEN** no invalid value is written and the rendered value remains at the minimum

#### Scenario: Increment at maximum
- **WHEN** a numeric preference is already at its maximum and increment is requested
- **THEN** no invalid value is written and the rendered value remains at the maximum

