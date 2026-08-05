## MODIFIED Requirements

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
