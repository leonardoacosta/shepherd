## ADDED Requirements

### Requirement: Authoritative integration pushes receive independent validation
Shepherd SHALL run its existing repository validation workflows for qualifying pushes to the authoritative `dev` integration branch without removing validation for pull requests, `master`, or the legacy `windows` CI branch.

#### Scenario: Source change lands directly on dev
- **WHEN** a non-Website-only source change is pushed to `dev`
- **THEN** CI schedules conventional-commit validation, maintenance validation, the Linux/macOS/Windows check matrix, and Windows package validation for that commit

#### Scenario: Website input lands directly on dev
- **WHEN** a path already covered by the Website workflow is pushed to `dev`
- **THEN** the Website build workflow validates that commit using its existing path filters

#### Scenario: Nix input lands directly on dev
- **WHEN** a path already covered by the Nix workflow is pushed to `dev`
- **THEN** the Nix flake workflow validates that commit using its existing path filters

#### Scenario: Pull request validation remains unchanged
- **WHEN** a pull request is opened, synchronized, or reopened against any currently supported target
- **THEN** the existing workflow pull-request behavior remains unchanged

### Requirement: Stable release preparation fails before wrong-branch mutation
Shepherd SHALL require the stable publication branch before either release preparation or release publication performs recipe work.

#### Scenario: Release command starts on dev
- **WHEN** a maintainer invokes release preparation or publication while the checked-out branch is not `master`
- **THEN** the shared release branch preflight exits nonzero before changelog generation, version edits, release commits, branch pushes, or tag creation

#### Scenario: Release command starts on master
- **WHEN** a maintainer invokes release preparation or publication from a clean `master` checkout
- **THEN** the branch preflight succeeds and the existing version, ancestry, documentation, validation, push, and tag checks continue to govern the release

### Requirement: Integration promotion is explicit and non-destructive
Shepherd SHALL document `dev` as the integration branch and `master` as the stable publication branch, with a fetched fast-forward-only promotion before stable release preparation.

#### Scenario: Master is an ancestor of the integration candidate
- **WHEN** the fetched `origin/master` is an ancestor of the fetched `origin/dev`
- **THEN** the documented promotion allows `master` to fast-forward to the exact integration candidate before validation and release

#### Scenario: Publication history has diverged
- **WHEN** `master` cannot fast-forward to the fetched `dev` candidate
- **THEN** the documented procedure stops and requires the publication history to be integrated into `dev` and revalidated
- **AND** it does not recommend force-push, history loss, or an automatic merge during release

### Requirement: Active OpenSpec work is valid and parser-visible
Shepherd SHALL keep active OpenSpec changes strictly valid, expose executable work as checkbox tasks, and archive completed work without erasing still-active owners.

#### Scenario: Legacy active changes are reconciled
- **WHEN** the approved planning-state repair completes
- **THEN** `windows-test-coverage` and `spike-remote-agent-catalog` pass strict validation and remain active with their incomplete tasks visible in `openspec list`

#### Scenario: Completed plugin lifecycle is accepted
- **WHEN** the user's approval accepts the documented conservative manual-only plugin drift outcome
- **THEN** the shipped command, migration regression, and decision are represented in a valid delta spec and completed task list
- **AND** the change is archived normally so its capability becomes canonical

#### Scenario: Validation covers the whole planning tree
- **WHEN** repository validation runs after reconciliation
- **THEN** all active changes and canonical specifications pass strict non-interactive OpenSpec validation
