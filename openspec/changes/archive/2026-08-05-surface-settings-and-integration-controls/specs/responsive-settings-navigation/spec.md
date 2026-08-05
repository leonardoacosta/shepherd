## ADDED Requirements

### Requirement: Settings section navigation adapts to available width
Settings SHALL compute section visibility and hit targets from one pure view model and SHALL keep the selected section visible at 40, 64, and 80 terminal columns.

#### Scenario: All labels fit
- **WHEN** the Settings header has enough width for every section and badge
- **THEN** sections render as Theme, Sound, Toast, Display, Behavior, Integrations, Experiments and each mouse target covers only its visible label

#### Scenario: Labels overflow
- **WHEN** all section labels do not fit
- **THEN** Settings shows bounded overflow navigation or a compact selected-section label and the selected section remains visible and clickable

#### Scenario: Section badge in a viewport
- **WHEN** a section with a badge enters or leaves the visible section viewport
- **THEN** the badge follows that section identity and does not alter another section's hit target

#### Scenario: Keyboard and mouse parity
- **WHEN** keyboard cycling and mouse selection target the same visible section
- **THEN** both select the same `SettingsSection` and apply the same selection normalization

### Requirement: Settings content uses stable row identity and scrolling
Every variable-length Settings section SHALL select rows by typed identity, maintain a normalized scroll offset, and use the same visible-row geometry for rendering and input.

#### Scenario: Selected row leaves viewport
- **WHEN** keyboard navigation moves selection beyond the current content viewport
- **THEN** the offset changes enough to keep the selected row visible

#### Scenario: Mouse selects a visible row
- **WHEN** the user clicks a rendered selectable row
- **THEN** hit-testing resolves the exact rendered row identity and no clipped row can be activated

#### Scenario: Rows change dynamically
- **WHEN** refresh, filtering, or a completed operation inserts, removes, or reorders rows
- **THEN** selection remains on the same identity when present or moves to the nearest valid selectable row when absent

#### Scenario: Terminal resizes
- **WHEN** Settings resizes between 40x20, 64x20, and 80x24
- **THEN** selected section/row identity is preserved and offsets normalize without rendering outside the popup

#### Scenario: Scrollbar interaction
- **WHEN** content exceeds the viewport
- **THEN** the shared scrollbar geometry represents the visible range and keyboard, wheel, click, and drag paths cannot select a different row than rendered

### Requirement: Settings degrades safely below its full support size
Settings SHALL remain non-panicking and closable below 40x20 even when it cannot expose the complete control surface.

#### Scenario: Extremely small terminal
- **WHEN** the terminal is smaller than the full 40x20 reachability contract
- **THEN** Settings renders a bounded compact state or nothing, never writes outside the frame, and retains an escape/close path
