## ADDED Requirements

### Requirement: Shipped CLI subcommands are documented before release

Every user-invocable CLI subcommand declared in the CLI spec SHALL be documented in the unreleased
CLI reference for each maintained locale before the release that ships it.

#### Scenario: Undocumented subcommand

- **WHEN** a subcommand is declared in `src/cli/spec.rs` and dispatched, but named in no
  `docs/next/website/src/content/docs/` locale
- **THEN** it is treated as an unmet release documentation requirement, not an optional follow-up

#### Scenario: Locale parity is not coverage

- **WHEN** every maintained locale is equally silent about a shipped subcommand
- **THEN** translation parity passing does not satisfy this requirement

### Requirement: Scriptable command contracts are stated explicitly

Documentation SHALL state the condition that produces a non-zero exit wherever a documented
command's exit code carries meaning beyond success or failure.

#### Scenario: Status-driven exit

- **WHEN** `plugin outdated` exits non-zero because at least one plugin reports the `outdated`
  status
- **THEN** the documentation states that condition alongside the status values, so the command can
  be used in a script without reading the source

#### Scenario: Enumerated status values

- **WHEN** a command reports one of a fixed set of status values
- **THEN** every value in that set is documented with its user-facing meaning
