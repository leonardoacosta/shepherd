## MODIFIED Requirements

### Requirement: Topbar renders configured rows and Agent tokens
An enabled desktop topbar SHALL render the nonempty rows declared by `ui.topbar.rows` using the
documented Agent-sidebar token vocabulary and styling rules. The topbar SHALL render as an
extension of the Agent sidebar rather than as an independent region: the sidebar SHALL own the
full-height left edge, and the topbar SHALL occupy the main-content width beside it.

#### Scenario: Sidebar anchors the full terminal height
- **WHEN** a desktop layout resolves at least one topbar row and the sidebar is visible
- **THEN** the sidebar rectangle spans from the first terminal row to the last
- **AND** the topbar rectangle begins at the sidebar's right edge

#### Scenario: Topbar reserves no sidebar geometry
- **WHEN** the topbar resolves no rows
- **THEN** the topbar rectangle is empty
- **AND** the sidebar rectangle still spans the full terminal height

#### Scenario: Multiple rows render independently
- **WHEN** two configured topbar rows each resolve at least one value
- **THEN** Shepherd renders two distinct topbar lines in configured order
- **AND** reserves two rows of main-content geometry beside the sidebar

#### Scenario: Built-in tokens use the active main pane
- **WHEN** the active workspace's main tab has a focused pane with state, tab, pane, agent, or terminal-title values
- **THEN** the matching topbar built-ins resolve from that pane
- **AND** auxiliary dock focus does not replace that main-pane context

#### Scenario: Custom tokens preserve workspace telemetry
- **WHEN** a `$name` token exists in active workspace metadata
- **THEN** the topbar renders the workspace value
- **AND** uses focused-main-pane metadata only when that workspace key is absent

#### Scenario: Styled and adjacent tokens render consistently
- **WHEN** a topbar row contains styled tokens and adjacent resolved values
- **THEN** Shepherd applies the configured foreground/bold/dim style
- **AND** uses the same state-icon and git-status separator rules as sidebar tokens where applicable

#### Scenario: Missing values elide cleanly
- **WHEN** a configured token has no value
- **THEN** Shepherd omits that token and its separator
- **AND** omits the complete row when none of its tokens resolve

#### Scenario: Short desktop retains body content
- **WHEN** resolved topbar rows would consume the full terminal height
- **THEN** Shepherd clips visible topbar rows to preserve at least one body row

#### Scenario: No workspace or mobile layout
- **WHEN** no active workspace exists or the client uses mobile layout
- **THEN** Shepherd reserves and renders no topbar rows

## ADDED Requirements

### Requirement: Chrome panels render on either side edge
Shepherd SHALL support a right chrome panel that renders configured token rows using the same
Agent-sidebar renderer, token vocabulary, and styling rules as the left sidebar. A chrome panel is
a token-driven region and SHALL NOT host a terminal runtime.

#### Scenario: Right panel renders configured rows
- **WHEN** a right chrome panel is enabled with rows that resolve at least one value
- **THEN** Shepherd reserves the configured width on the right edge
- **AND** renders those rows using the Agent-sidebar token vocabulary

#### Scenario: Right panel elides absent values
- **WHEN** a configured right-panel token has no value
- **THEN** Shepherd omits that token and its separator
- **AND** omits the complete row when none of its tokens resolve

#### Scenario: Both side panels enabled
- **WHEN** the left sidebar and the right chrome panel are both visible
- **THEN** both span the full terminal height
- **AND** Shepherd clamps their widths to preserve at least one main-content column

#### Scenario: Right panel disabled or unresolvable
- **WHEN** the right chrome panel is disabled or resolves no rows
- **THEN** its rectangle is empty
- **AND** the main content retains the otherwise reserved width

### Requirement: Dock is a terminal host distinct from chrome panels
The spec SHALL describe the dock as an auxiliary terminal region and SHALL NOT present it as a
chrome panel. A dock hosts a live terminal runtime and routes focus to it; a chrome panel renders
resolved tokens and accepts no focus.

#### Scenario: Dock and right chrome panel coexist
- **WHEN** a right-side dock and a right chrome panel are both enabled
- **THEN** Shepherd reserves geometry for both on that edge
- **AND** clamps their combined size to preserve at least one main-content column

#### Scenario: Chrome panel rejects terminal semantics
- **WHEN** a chrome panel region is computed
- **THEN** Shepherd allocates no terminal runtime for it
- **AND** the panel takes no part in focus routing or backing-tab state
