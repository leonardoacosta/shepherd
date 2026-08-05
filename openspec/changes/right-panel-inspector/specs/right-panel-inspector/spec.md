## ADDED Requirements

### Requirement: Each inspector tab declares its own token rows
The right panel SHALL carry a configured tab list declaring tab order, enablement, and the start
tab, and each declared tab SHALL own its token rows in its own configuration sub-table. The panel
SHALL NOT carry panel-level token rows.

#### Scenario: A configured tab renders its own rows
- **WHEN** the panel declares a tab and that tab's sub-table declares rows
- **THEN** Shepherd renders that tab's content using its own declared rows
- **AND** applies any configured foreground, bold, or dim style to each token

#### Scenario: Tab order and start tab come from the tab list
- **WHEN** the panel declares more than one tab
- **THEN** Shepherd presents them in the declared order
- **AND** selects the first declared tab on start

#### Scenario: Unknown tab name is rejected
- **WHEN** the configured tab list names a tab that is not in the documented vocabulary
- **THEN** Shepherd reports a configuration error and does not start with that tab list

#### Scenario: A token used in the wrong tab is rejected
- **WHEN** a tab's rows declare a token belonging to a different tab's vocabulary
- **THEN** Shepherd reports a configuration error naming the token and the tab
- **AND** does not start with that configuration

#### Scenario: Absent values elide as they do elsewhere
- **WHEN** a declared token resolves to no value
- **THEN** Shepherd omits that token and its separator
- **AND** omits the complete row when none of its tokens resolve

### Requirement: The tab affordance resolves against available width
Shepherd SHALL resolve the tab affordance from the panel's available content width. Below the
width that fits a tab bar, Shepherd SHALL name the selected tab and its position rather than
render clipped tab labels.

#### Scenario: Width cannot fit a tab bar
- **WHEN** the panel's content width is narrower than the declared tab labels require
- **THEN** Shepherd renders a header naming the selected tab and its position in the tab list
- **AND** renders no clipped tab labels

#### Scenario: Width fits a tab bar
- **WHEN** the panel's content width fits the declared tab labels
- **THEN** Shepherd renders the tab labels in declared order
- **AND** distinguishes the selected tab from the others

#### Scenario: Selected tab is identifiable at every width
- **WHEN** the panel renders at any width that reserves geometry
- **THEN** the selected tab is identifiable from the rendered output

### Requirement: The inspector accepts pointer interaction without terminal input
A user SHALL be able to change the selected tab and scroll the selected tab's content with the
pointer. The inspector SHALL NOT receive terminal input, allocate a terminal runtime, or change
the focused pane.

#### Scenario: Clicking a tab changes the selection
- **WHEN** a user clicks a rendered tab label
- **THEN** Shepherd selects that tab and renders its content
- **AND** the active workspace, tab, and focused pane are unchanged

#### Scenario: Scrolling the body scrolls the selected tab
- **WHEN** a user scrolls within the panel's content region
- **THEN** Shepherd scrolls the selected tab's content
- **AND** does not scroll the focused pane

#### Scenario: Scroll position is per tab
- **WHEN** a user scrolls one tab, selects another, and returns to the first
- **THEN** the first tab renders at its previous scroll position

#### Scenario: Typing reaches the focused pane
- **WHEN** the inspector is visible with a tab selected and a user types
- **THEN** the keystroke reaches the focused pane
- **AND** the selected tab and its scroll position are unchanged

### Requirement: The proposals tab lists a checkout's open change proposals
Shepherd SHALL provide a proposals tab that lists the open change proposals of the checkout the
active space resolves to, with each proposal's completed and total task counts. It SHALL reuse the
existing per-checkout project-status refresh rather than starting a second one.

#### Scenario: Proposals resolve for the active space
- **WHEN** the proposals tab is selected and the active space's checkout has open change proposals
- **THEN** Shepherd lists them with their task counts

#### Scenario: Checkout has no proposals
- **WHEN** the proposals tab is selected and the active space's checkout has no change proposals
- **THEN** Shepherd renders the tab with no entries, no placeholder, and no error surface

#### Scenario: Provider is unavailable
- **WHEN** the proposals provider is missing, exits non-zero, times out, or returns an
  unparseable body
- **THEN** the tab renders no entries
- **AND** Shepherd surfaces no error dialog and writes no user-visible warning

#### Scenario: No configured consumer starts no refresh
- **WHEN** no configured space row and no configured inspector tab consumes proposal status
- **THEN** Shepherd never starts a project-status refresh for it
- **AND** spawns no provider process

#### Scenario: Tab and space rows share one refresh
- **WHEN** both a configured space row and the proposals tab consume proposal status for one
  checkout
- **THEN** Shepherd performs one refresh for that checkout
- **AND** both surfaces render the same resolved values

#### Scenario: Refresh runs off the render path
- **WHEN** a refresh serving the proposals tab is due
- **THEN** Shepherd performs the provider work without blocking a frame
- **AND** renders the previously cached values until the refresh completes
