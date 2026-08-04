## Context

The remote catalog is string-shaped at rest, but resolution remains closed. `parse_catalog` in `src/detect/manifest_update.rs` calls `parse_agent_label` for each catalog id and warns while dropping an unresolved entry. The repository-side catalog validator independently requires every catalog id to correspond to a bundled manifest. Remote data can therefore revise detection rules for a released agent but cannot introduce a new agent identity.

The earlier coupling inventory found that API schemas and hook-originated labels are already strings, while screen detection remains deeply `Agent`-typed: canonical parsing and labels, process identification, `ManifestCache`, pane lifecycle, terminal detected-agent state, sound/sidebar configuration, and CLI enumeration all rely on the enum. Moving the remote gate alone would create an unresolved identity that the rest of that path cannot safely carry.

Manifest input already has structural caps for rules, gate depth, gate count, matcher count, and matcher length, and uses Rust's linear-time regex engine. Those controls bound resource use but cannot decide whether a matcher is semantically over-broad, a state label is misleading, or the selected screen evidence is incidental. Human content review and representative fixtures remain necessary.

## Goals / Non-Goals

**Goals:**

- Give contributors a clear path to propose support for a new curated agent.
- Require bottom-buffer screen evidence and representative state fixtures.
- Expose both automated structural validation and human matcher review in the acceptance checklist.
- Preserve the existing closed remote identity boundary.

**Non-Goals:**

- Adding a string-id detection path alongside or instead of `Agent`.
- Allowing the remote catalog to introduce an agent not shipped in the binary.
- Treating structural caps as proof that matcher semantics are correct.

## Decisions

### Choose the community pull-request path

The accepted path adds a submission guide, focused pull-request checklist, and fixture-backed acceptance criterion. It delivers the practical contribution outcome—“Shepherd recognizes my agent”—through reviewable repository changes and a release, without changing runtime identity or trust boundaries.

Rejected: moving to open remote string identities in this change. That alternative reaches into process identification, manifest cache and loading, terminal/pane lifecycle, per-agent settings, and CLI enumeration even though wire and hook paths are already strings.

### Require evidence from the detector's authoritative source

Submissions capture representative bottom-buffer states using `shepherd agent read <pane> --source detection --format text`; ANSI evidence is added when style or alternate-screen behavior matters. Fixtures and rules identify invariant controls and explicit alternatives rather than matching incidental whole-pane content.

### Combine automated bounds with human semantic review

The existing manifest check remains the structural gate. The pull-request checklist separately requires a maintainer to inspect matcher intent, false-positive risk, state labels, and the relationship between each fixture and its expected classification.

### Keep delivery curated and release-time

An accepted submission adds the necessary known identity and bundled manifest. The website catalog may update that known agent after release, but an unknown catalog id continues to be rejected both in the runtime and repository validator.

## Risks / Trade-offs

- **Contribution still waits for a release** → State this explicitly in the guide; the reduced architectural and trust risk is the chosen trade-off.
- **Fixture data can capture incidental UI text** → Require minimal bottom-buffer evidence and review invariant controls versus alternatives.
- **A valid regex can still classify too broadly** → Make semantic matcher review a named acceptance step.
- **A new fixture harness can grow into a large agent-specific suite** → Keep it to representative classification evidence; retain Rust tests for rule semantics and manifest infrastructure.

## Migration Plan

1. Establish the minimal representative-fixture format and acceptance test.
2. Publish the contributor guide with capture, manifest, cap, and validation instructions.
3. Add the focused pull-request checklist and human review gate.
4. Validate the fixture path, manifest checker, and repository-wide checks.

## Open Questions

None. Reconsidering open remote string identities requires a separate architecture proposal starting from the recorded coupling and trust analysis.
