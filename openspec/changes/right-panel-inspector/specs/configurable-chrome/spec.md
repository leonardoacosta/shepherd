## MODIFIED Requirements

### Requirement: Chrome panels render on either side edge
Shepherd SHALL support a right chrome panel that spans the full terminal height on the right edge
and hosts the right-panel inspector's tab surface. A chrome panel SHALL NOT host a terminal
runtime. Its geometry SHALL follow its configured tabs rather than the values its content
resolves, so a tab whose provider has returned nothing still owns its width.

#### Scenario: Right panel reserves width for configured tabs
- **WHEN** a right chrome panel is enabled and declares at least one tab
- **THEN** Shepherd reserves the configured width on the right edge
- **AND** renders the selected tab's content in that region

#### Scenario: Reserved width does not depend on resolved content
- **WHEN** a right chrome panel is enabled with a tab whose content resolves no value
- **THEN** Shepherd still reserves the configured width
- **AND** renders the tab with no placeholder and no error surface

#### Scenario: Both side panels enabled
- **WHEN** the left sidebar and the right chrome panel are both visible
- **THEN** both span the full terminal height
- **AND** Shepherd clamps their widths to preserve at least one main-content column

#### Scenario: Right panel disabled or declares no tabs
- **WHEN** the right chrome panel is disabled or its configured tab list is empty
- **THEN** its rectangle is empty
- **AND** the main content retains the otherwise reserved width

### Requirement: Dock is a terminal host distinct from chrome panels
The spec SHALL describe the dock as an auxiliary terminal region and SHALL NOT present it as a
chrome panel. A dock hosts a live terminal runtime and routes terminal input to it; a chrome panel
hosts no terminal runtime and never receives terminal input, while accepting pointer interaction
on the same terms as the left sidebar.

#### Scenario: Dock and right chrome panel coexist
- **WHEN** a right-side dock and a right chrome panel are both enabled
- **THEN** Shepherd reserves geometry for both on that edge
- **AND** clamps their combined size to preserve at least one main-content column

#### Scenario: Chrome panel rejects terminal semantics
- **WHEN** a chrome panel region is computed
- **THEN** Shepherd allocates no terminal runtime for it
- **AND** the panel takes no part in terminal input routing or backing-tab state

#### Scenario: Chrome panel accepts pointer interaction
- **WHEN** a user clicks or scrolls within a chrome panel region
- **THEN** Shepherd routes that pointer event to the panel
- **AND** the active tab, focused pane, and terminal input destination are unchanged

#### Scenario: Keystrokes bypass a visible chrome panel
- **WHEN** a chrome panel is visible and a user types
- **THEN** the keystroke reaches the focused pane
- **AND** the panel consumes no keystroke
