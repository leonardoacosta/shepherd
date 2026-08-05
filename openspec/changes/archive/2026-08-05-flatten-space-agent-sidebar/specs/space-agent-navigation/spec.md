## ADDED Requirements

### Requirement: The sidebar renders one flat space-and-agent list
The expanded sidebar SHALL render a single list in which each space row is followed directly by
the agent rows belonging to that space. It SHALL NOT render a second section, a section divider,
or a tab row at any depth.

#### Scenario: Agents render beneath their own space
- **WHEN** two spaces are open and each is running agents
- **THEN** each space row is followed by its own agent rows, in space order
- **AND** no agent row appears beneath a space it does not belong to

#### Scenario: Tabs are not rows
- **WHEN** a space has more than one tab, each carrying an agent
- **THEN** every agent renders as a row directly beneath that space
- **AND** no row represents a tab

#### Scenario: No section divider is drawn
- **WHEN** the expanded sidebar renders
- **THEN** no draggable divider is drawn and no divider hit target is registered

### Requirement: An agent row is the switch target
Selecting an agent row SHALL activate the space, tab, and pane that row identifies, regardless of
which space is active when the row is selected.

#### Scenario: Selecting an agent in an inactive space
- **WHEN** an agent row belonging to a space that is not active is selected
- **THEN** Shepherd activates that space, that agent's tab, and that agent's pane

#### Scenario: Selecting an agent in a non-active tab
- **WHEN** an agent row belonging to a non-active tab of the active space is selected
- **THEN** Shepherd activates that tab and focuses that agent's pane

#### Scenario: Shortcut numbers follow visible position
- **WHEN** rows are assigned shortcut numbers
- **THEN** numbers are assigned by visible list position across the whole merged list
- **AND** a number is never repeated across spaces

### Requirement: A space running no agent shows an explicit empty state
A space with no detected agent SHALL render one empty-state row beneath its space row. The row
SHALL NOT be selectable as an agent.

#### Scenario: Space with no detected agent
- **WHEN** a space is open and none of its panes has a detected agent
- **THEN** exactly one empty-state row renders beneath that space row
- **AND** selecting it does not change the focused pane

#### Scenario: Empty state is distinct from collapse
- **WHEN** one space is collapsed and another is running no agent
- **THEN** the collapsed space shows no child rows
- **AND** the agentless space shows its empty-state row

### Requirement: The merge preserves existing space-list behaviour
Worktree grouping, space collapse, space drag-reorder, and scrolling SHALL behave in the merged
list as they did in the space section.

#### Scenario: Worktree children keep their grouping
- **WHEN** a space has Shepherd-managed worktree children
- **THEN** those children render indented beneath their parent space
- **AND** the parent's collapse chevron still collapses them

#### Scenario: Scrolled rows remain clickable
- **WHEN** the merged list is taller than the sidebar and has been scrolled
- **THEN** a click maps to the row rendered at that position

#### Scenario: Reordering spaces moves their agents
- **WHEN** a space is dragged to a new position
- **THEN** its agent rows move with it and remain beneath it

### Requirement: The section split preference is removed
Shepherd SHALL NOT expose, persist, or mirror a sidebar section split. A session file written
before its removal SHALL restore without error.

#### Scenario: Older session file restores
- **WHEN** a session snapshot carrying a section-split value is restored
- **THEN** Shepherd restores the session and ignores that value
- **AND** reports no error

#### Scenario: Client projection omits the field
- **WHEN** the client view state is projected
- **THEN** it carries no section-split field
