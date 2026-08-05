# presentation-layout-preset-decision Specification

## Purpose
TBD - created by archiving change research-presentation-layout-presets. Update Purpose after archive.
## Requirements
### Requirement: Preset candidates have production-equivalent evidence
The research change SHALL record three to five exact candidates for each evaluated topbar, Agent-row, and Space-row surface and SHALL show their production-equivalent output at every required width.

#### Scenario: Sidebar widths
- **WHEN** an Agent-row or Space-row candidate is evaluated
- **THEN** its exact rows and rendered cells are recorded at 18, 24, and 36 columns with representative long and missing values

#### Scenario: Topbar widths
- **WHEN** a topbar candidate is evaluated
- **THEN** its exact rows and rendered cells are recorded at 40 and 80 columns with representative long and missing values

#### Scenario: Production rules
- **WHEN** evidence is generated
- **THEN** focused test-only characterization verifies it against production token resolution, style, elision, and truncation behavior and records the command and source revision

### Requirement: Custom layouts are classified conservatively
The decision contract SHALL classify a layout as custom when it contains styled tokens, `$custom` tokens, nonzero applicable gaps, per-agent overrides, or any unrecognized future authoring field.

#### Scenario: Advanced field exists
- **WHEN** any custom-classifying field is present even if visible output resembles a preset
- **THEN** the layout is classified as custom and no implicit replacement is allowed

#### Scenario: Exact preset match
- **WHEN** normalized rows exactly match a known preset and no custom-classifying field is present
- **THEN** the decision artifact classifies the layout as that preset

### Requirement: Replacement semantics preserve authored configuration
The decision contract SHALL require exact-row preview and explicit confirmation before replacing a custom layout, and SHALL preserve Agent `rows_by_agent` unless a separate named opt-in clears it.

#### Scenario: Cancel replacement
- **WHEN** the user cancels a preset preview or replacement confirmation
- **THEN** no configuration content is changed

#### Scenario: Replace custom base rows
- **WHEN** the user explicitly confirms replacement of custom base rows without opting to clear per-agent overrides
- **THEN** only the previewed base rows are eligible for replacement and `rows_by_agent` remains byte-for-byte semantically equivalent

#### Scenario: Clear per-agent overrides
- **WHEN** a future flow offers clearing `rows_by_agent`
- **THEN** the confirmation names that additional loss separately and requires an explicit opt-in

### Requirement: Research ends with an explicit disposition
The research change SHALL record either an approved exact preset contract suitable for a bounded follow-on feature or a reason that TOML remains sufficient.

#### Scenario: Candidates approved
- **WHEN** the terminal evidence gate approves candidate contents and replacement semantics
- **THEN** the final design names every approved preset and contains no unresolved implementation judgment

#### Scenario: No picker warranted
- **WHEN** the evidence does not support a useful safe candidate set
- **THEN** the final design records that no picker feature will be authored and why

