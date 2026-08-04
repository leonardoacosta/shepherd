# configurable-chrome Specification

## Purpose
TBD - created by archiving change surface-configurable-chrome. Update Purpose after archive.
## Requirements
### Requirement: Display settings surface configurable chrome
Shepherd SHALL expose desktop topbar and dock presentation through the existing Settings experience without requiring users to edit TOML for basic enablement and dock placement.

#### Scenario: Current chrome values are visible
- **WHEN** a user opens the Display Settings section
- **THEN** Shepherd shows the current agent-border-label, topbar-enabled, dock-enabled, dock-side, and dock-size values
- **AND** Shepherd shows the configured topbar row count and identifies row contents as advanced config-file editing

#### Scenario: Keyboard and mouse actions persist the same setting
- **WHEN** a user changes a Display setting using either its keyboard or mouse affordance
- **THEN** Shepherd writes only the corresponding `ui`, `ui.topbar`, or `ui.dock` key through the existing config writer
- **AND** applies the successfully parsed config live without changing unrelated config text

#### Scenario: Dock size remains renderable
- **WHEN** a user decrements dock size in Settings or loads a configured size
- **THEN** Shepherd SHALL accept only sizes of at least three cells
- **AND** SHALL surface a config diagnostic for a smaller value rather than rendering an unusable bordered dock

#### Scenario: Display settings are opened on a narrow layout
- **WHEN** Settings is available while the client uses mobile layout
- **THEN** the Display section SHALL identify topbar and dock effects as desktop-only
- **AND** changing them SHALL NOT add mobile topbar or dock geometry

### Requirement: Topbar renders configured rows and Agent tokens
An enabled desktop topbar SHALL render the nonempty rows declared by `ui.topbar.rows` using the documented Agent-sidebar token vocabulary and styling rules.

#### Scenario: Multiple rows render independently
- **WHEN** two configured topbar rows each resolve at least one value
- **THEN** Shepherd renders two distinct topbar lines in configured order
- **AND** reserves two rows of desktop geometry

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

### Requirement: Dock geometry follows live workspace occupancy
Shepherd SHALL reserve the configured full dock region only when the dock is enabled and the active workspace owns a valid dock pane with a live terminal runtime.

#### Scenario: Enabled empty dock preserves terminal space
- **WHEN** `ui.dock.enabled` is true and the active workspace has no live dock pane
- **THEN** the dock rectangle is empty
- **AND** the main terminal retains the otherwise reserved region

#### Scenario: Live bottom or right dock reserves bounded geometry
- **WHEN** the active workspace has a live dock and side is `bottom` or `right`
- **THEN** Shepherd reserves the configured size on that edge
- **AND** clamps it to preserve at least one main-content row or column
- **AND** resizes the dock runtime to the bordered region's inner dimensions during view computation rather than render

#### Scenario: Terminal cannot fit a minimum bordered dock and main body
- **WHEN** a live dock is enabled but the available edge cannot fit both three dock cells and one main-content cell
- **THEN** the dock rectangle is empty and the main body remains usable
- **AND** Shepherd does not resize the dock runtime to a zero-row or zero-column interior

#### Scenario: Disabled dock keeps its process but hides presentation
- **WHEN** a user disables the dock while its pane is running
- **THEN** Shepherd clears dock focus and reserves no dock geometry
- **AND** does not terminate the pane
- **AND** reveals the same live pane when the user re-enables the dock

#### Scenario: Close or runtime loss releases geometry
- **WHEN** the dock pane exits, closes, is removed with its workspace/plugin, or loses its runtime
- **THEN** Shepherd clears effective occupancy and dock focus
- **AND** restores the full main area

#### Scenario: Workspace switch scopes occupancy
- **WHEN** a user switches between a workspace with a live dock and one without a live dock
- **THEN** only the workspace with the live dock reserves dock geometry
- **AND** dock focus does not leak between workspaces

#### Scenario: Opening a dock while disabled is rejected
- **WHEN** any caller requests a `placement = "dock"` pane while dock enablement is false
- **THEN** the existing plugin pane API returns `plugin_dock_disabled`
- **AND** creates no hidden tab, pane, terminal, runtime, or process

#### Scenario: Stale and occupied records are reconciled
- **WHEN** a new dock open finds a stale record without a live pane/runtime
- **THEN** Shepherd removes or reconciles the stale record before opening
- **BUT WHEN** a live dock already occupies the workspace
- **THEN** Shepherd returns `plugin_dock_occupied` and launches nothing else

### Requirement: Dock panes are explicitly discoverable
Display Settings SHALL enumerate dock-capable pane manifests without automatically running plugin code.

#### Scenario: Eligible pane choices are stable
- **WHEN** installed plugins include pane manifests
- **THEN** Display lists only enabled, effective-platform-supported panes whose placement is `dock`
- **AND** sorts them deterministically by plugin name, pane title, plugin ID, and entrypoint ID

#### Scenario: No dock pane is available
- **WHEN** no eligible dock pane exists
- **THEN** Display shows a clear empty-state explanation and no open action

#### Scenario: User explicitly opens a dock pane
- **WHEN** dock enablement is true, the active workspace is unoccupied, and the user activates an eligible choice with Enter or click
- **THEN** Shepherd invokes the existing plugin pane-open path exactly once for that workspace and entrypoint
- **AND** preserves the workspace's active main tab
- **AND** closes Settings on success so the newly focused dock is visible

#### Scenario: Dock launch fails
- **WHEN** explicit dock opening returns a disabled, unsupported, occupied, registry, environment, or process-launch error
- **THEN** Settings remains open
- **AND** shows bounded actionable feedback
- **AND** leaves no partial dock record or unexplained reserved geometry

### Requirement: Dock focus routes input without replacing the main tab
A live desktop dock SHALL be an interactive auxiliary focus target that does not change the workspace's active main tab.

#### Scenario: Clicking the dock focuses it in place
- **WHEN** the user presses inside a live dock's inner rectangle
- **THEN** Shepherd marks the dock as the client input focus
- **AND** keeps the workspace and active main tab unchanged
- **AND** renders the dock border with the existing focused-surface accent language

#### Scenario: Shared pane APIs cannot replace client dock focus
- **WHEN** `pane.focus` or `plugin.pane.focus` targets a live dock pane
- **THEN** Shepherd returns `pane_not_focusable`
- **AND** keeps the workspace's active main tab and client auxiliary focus unchanged

#### Scenario: Dock keys preserve Shepherd shortcuts
- **WHEN** dock focus is valid and the user presses a terminal key
- **THEN** Shepherd first applies existing prefix and direct TUI shortcut interception
- **AND** otherwise forwards encoded bytes to the dock runtime

#### Scenario: Key release returns to the press owner
- **WHEN** a key press was forwarded to the dock and presentation focus changes or the dock is hidden before its release
- **THEN** Shepherd forwards the release to the same terminal input target while that runtime remains live
- **AND** does not send it to the newly focused main pane

#### Scenario: Key release loses its runtime
- **WHEN** a key press was forwarded to the dock and its runtime exits before release
- **THEN** Shepherd drops the release
- **AND** does not redirect it to an unrelated terminal

#### Scenario: Dock mouse coordinates are translated
- **WHEN** the dock application has enabled mouse reporting and the user clicks, drags, moves, or scrolls inside the dock
- **THEN** Shepherd translates the event relative to the dock inner rectangle and forwards it to the dock runtime

#### Scenario: Dock supports host selection and scrollback
- **WHEN** the dock application has not claimed a mouse gesture or plain page-scroll behavior
- **THEN** Shepherd applies the existing pane selection, copy-on-select, scrollbar, and scrollback rules to the dock pane

#### Scenario: Main interaction clears dock focus
- **WHEN** the user clicks an ordinary pane or directly navigates workspace, tab, or pane focus
- **THEN** Shepherd clears dock focus before routing subsequent input to the main surface

#### Scenario: Dock focus remains client-only
- **WHEN** one TUI client focuses a dock or an API client focuses an ordinary main identity
- **THEN** auxiliary dock focus remains isolated to the originating TUI client
- **AND** dock focus does not emit shared workspace/tab/pane focus events or terminal focus-in/focus-out sequences

#### Scenario: Paste follows auxiliary input focus
- **WHEN** a user pastes while a live dock has auxiliary focus
- **THEN** Shepherd sends the paste to that dock runtime rather than the main focused pane

#### Scenario: Invalid dock focus fails closed to the main surface
- **WHEN** dock focus exists but the dock becomes disabled, nonlive, closed, stale, or belongs to a different active workspace
- **THEN** Shepherd clears dock focus and sends no bytes to the invalid target
- **AND** subsequent ordinary input uses the active main pane, except that an outstanding release still follows its separate live press owner

### Requirement: Dock backing tabs stay out of ordinary TUI tab state
Shepherd SHALL retain a live dock's backing tab for existing runtime/API identity while excluding that tab from ordinary TUI tab presentation, navigation, and disk session restore.

#### Scenario: Desktop and mobile tab surfaces hide the backing tab
- **WHEN** a workspace owns a dock backing tab
- **THEN** desktop tab chrome, navigator rows, and mobile tab lists omit it
- **AND** visible fallback ordinals contain no ghost entry or numbering gap

#### Scenario: Tab navigation skips the backing tab
- **WHEN** a user cycles tabs, scrolls the tab bar, or uses an indexed tab shortcut
- **THEN** Shepherd selects only visible non-dock tabs
- **AND** preserves the dock process and auxiliary presentation

#### Scenario: Backing identity survives tab index shifts
- **WHEN** a non-dock tab before the backing tab closes, moves, or is inserted
- **THEN** Shepherd locates dock ownership from the stable pane ID and current containing tab
- **AND** never hides, renders, snapshots, or routes input to the wrong tab

#### Scenario: Backing tab cannot replace the main tab through API focus
- **WHEN** `tab.focus` targets a live dock backing tab
- **THEN** Shepherd returns `tab_not_focusable`
- **AND** leaves the active main tab and auxiliary dock state unchanged

#### Scenario: Backing topology remains an isolated container
- **WHEN** a pane split, pane move, swap, or layout operation would add, remove, or re-home panes through a managed dock backing tab
- **THEN** Shepherd rejects the operation before mutation
- **AND** the backing tab remains a workspace-local single-pane container without losing ordinary panes or processes

#### Scenario: Closing workspace cleans up the dock
- **WHEN** a workspace containing a live dock is closed
- **THEN** Shepherd closes the backing pane/runtime through existing workspace cleanup
- **AND** removes dock focus and ownership records

#### Scenario: Disk session snapshots omit dock execution
- **WHEN** Shepherd captures structural or screen-history state for disk persistence while a dock is running
- **THEN** the snapshot omits the dock backing tab and its pane/tab public-number entries
- **AND** preserves all non-dock tab layouts, active-tab mapping, identities, and monotonic next-number counters

#### Scenario: Restore does not auto-run dock plugin code
- **WHEN** a session captured with a live dock is restored
- **THEN** no dock backing tab or plugin process is recreated from that snapshot
- **AND** the user may explicitly reopen an eligible dock from Display Settings

#### Scenario: Unix live handoff preserves the running dock
- **WHEN** a Unix server performs compatible live handoff while a dock is running
- **THEN** Shepherd transfers the backing tab and runtime through the handoff-only capture path
- **AND** rebinds dock ownership after pane-ID remapping from optional private handoff metadata
- **AND** does not change the public wire protocol or persisted snapshot format version

#### Scenario: Neutral live APIs retain current identity
- **WHEN** an API client lists or gets live tabs or panes while a dock is running
- **THEN** the backing tab and pane remain available through the existing neutral list/get and event contracts
- **AND** no wire schema or protocol version changes

### Requirement: Configurable chrome is documented on the unreleased path
Shepherd SHALL make disabled-by-default chrome discoverable through executable default configuration and unreleased documentation without modifying stable release docs.

#### Scenario: Default config contains valid examples
- **WHEN** a user runs `shepherd --default-config`
- **THEN** the output contains disabled topbar and dock example sections with explanatory comments
- **AND** those examples parse through the current config model

#### Scenario: Preview docs describe the complete contract
- **WHEN** a user reads the preview configuration docs or config reference
- **THEN** the docs explain desktop-only layout, token sources and rows, minimum/bounded dock size, eligible pane opening, auxiliary focus, hidden/non-restored backing tabs, and errors
- **AND** distinguish disk restore from compatible Unix live-handoff preservation
- **AND** distinguish Shepherd presentation from external metadata-producer lifecycle

#### Scenario: Documentation stays release-correct
- **WHEN** the feature is authored before release
- **THEN** English, Japanese, and Chinese preview configuration pages and `docs/next/CHANGELOG.md` are updated together
- **AND** stable website docs, root README, root changelog, and `website/latest.json` remain unchanged

