# community-agent-manifest-submissions Specification

## Purpose
TBD - created by archiving change spike-remote-agent-catalog. Update Purpose after archive.
## Requirements
### Requirement: Agent submissions include authoritative detection evidence
Shepherd SHALL document a community submission path that requires representative bottom-buffer evidence and expected classifications for each proposed agent state.

#### Scenario: Contributor proposes a new agent
- **WHEN** a contributor submits a new bundled agent-detection manifest
- **THEN** the submission includes captured detection-source text for representative visible states
- **AND** it includes ANSI evidence when styling or alternate-screen behavior affects the rule

#### Scenario: A state has multiple valid visible controls
- **WHEN** an agent presents alternative invariant controls for the same state
- **THEN** the fixture and manifest represent those alternatives as explicit OR paths
- **AND** incidental whole-pane text is not used as the detection authority

### Requirement: Submission review covers structure and matcher meaning
Shepherd SHALL require both automated manifest validation and explicit human review of matcher semantics before accepting a community agent manifest.

#### Scenario: Automated submission checks run
- **WHEN** a contributor validates a proposed manifest
- **THEN** the repository's manifest checker enforces manifest shape and the configured rule, gate, matcher-count, and matcher-length limits

#### Scenario: Maintainer reviews a structurally valid manifest
- **WHEN** automated checks accept the manifest syntax and bounds
- **THEN** the review checklist still requires examination of false-positive risk, matcher intent, state labels, and fixture-to-classification coverage

### Requirement: New agent identities remain curated and bundled
Shepherd SHALL continue rejecting remote catalog ids that are unknown to the released binary; community submissions become available through reviewed bundled changes and a release.

#### Scenario: Remote catalog contains an unknown id
- **WHEN** a catalog entry cannot resolve through the binary's known agent labels
- **THEN** runtime catalog parsing rejects the entry
- **AND** repository catalog validation rejects publishing that entry

#### Scenario: Community submission is accepted
- **WHEN** the manifest, identity integration, fixtures, and review checklist are complete
- **THEN** the new agent ships as a curated bundled identity in a Shepherd release
- **AND** subsequent remote catalog updates may target it only after the released binary knows the id

