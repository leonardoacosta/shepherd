# truthful-user-guidance Specification

## Purpose
TBD - created by archiving change align-onboarding-and-hermes-guidance. Update Purpose after archive.
## Requirements
### Requirement: Onboarding renders effective shortcuts
The onboarding welcome surface SHALL derive its prefix, help, and Settings shortcut labels from the effective application keybinding configuration using the shared human-readable formatting rules.

#### Scenario: Default bindings
- **WHEN** onboarding renders with the default keybinding configuration
- **THEN** it identifies the effective default prefix, help, and Settings actions without separate hard-coded shortcut literals

#### Scenario: Customized bindings
- **WHEN** prefix, help, or Settings bindings differ from their defaults
- **THEN** onboarding renders the customized labels and does not render the displaced default as the instruction

#### Scenario: Unset action
- **WHEN** help or Settings has no effective binding
- **THEN** onboarding labels that action as unset instead of claiming an unusable shortcut

#### Scenario: Multiple alternatives
- **WHEN** an action has multiple configured bindings
- **THEN** onboarding preserves every alternative represented by the shared action label

### Requirement: Onboarding keeps shortcut guidance readable at narrow widths
The onboarding welcome surface SHALL allocate and wrap a semantic shortcut block so every effective shortcut remains readable at the supported 40-column width.

#### Scenario: Forty-column custom labels
- **WHEN** onboarding renders at 40 columns with longer customized shortcut labels
- **THEN** the complete prefix, help, and Settings labels appear across bounded lines without truncation or overlap with the continue action

#### Scenario: Render remains pure
- **WHEN** onboarding is rendered repeatedly from unchanged `AppState`
- **THEN** the same cells are produced and no selection, keybinding, or onboarding state is mutated

### Requirement: Next Hermes guidance matches runtime authority
Every maintained unreleased documentation locale SHALL describe Hermes as reporting session identity while screen detection remains the source of agent state, and SHALL show the current bundled Hermes integration version.

#### Scenario: Session-only authority
- **WHEN** a user reads the next-version Hermes agent or integration documentation
- **THEN** it does not claim that Hermes reports lifecycle state or disables screen detection

#### Scenario: Current version
- **WHEN** the bundled Hermes integration version changes before release
- **THEN** documentation parity validation detects a stale version in any maintained locale

#### Scenario: Stable documentation boundary
- **WHEN** this unreleased correction is implemented
- **THEN** stable documentation remains unchanged unless its released runtime contract is separately verified

