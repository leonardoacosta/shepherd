## 1. Preserve the accepted architecture decision

- [x] 1.1 Verify that `parse_catalog` still drops ids that do not resolve through `parse_agent_label`, and record the current enum-typed detection/lifecycle surfaces, string-native API/hook surfaces, structural manifest caps, linear-regex guarantee, and semantic review gap in `design.md`. Expected result: the decisive runtime gate and real coupling boundary are documented against current source.
  - touches: `openspec/changes/spike-remote-agent-catalog/design.md`
  - depends on: []

- [x] 1.2 Select the community pull-request path and reject the full remote string-id architecture for this increment. Expected result: the design preserves curated bundled identities while defining the contribution outcome and trust boundary.
  - touches: `openspec/changes/spike-remote-agent-catalog/proposal.md`, `openspec/changes/spike-remote-agent-catalog/design.md`
  - depends on: 1.1

## 2. Build the submission path

- [ ] 2.1 Add the smallest representative screen-fixture acceptance path for a community manifest, covering expected classifications without introducing a large agent-specific full-screen suite. Run the focused manifest test target; expected result: the fixture proves accepted state classification from detection-source evidence.
  - touches: `tests/fixtures/agent-screens/` or the nearest existing fixture location, `src/detect/manifest/tests.rs`
  - depends on: 1.2

- [ ] 2.2 Add `website/agent-detection/SUBMITTING.md` with capture commands, manifest fields, structural caps, fixture format, local validation, release-time delivery, and maintainer semantic-review expectations. Expected result: a contributor can produce a complete submission without inferring hidden acceptance criteria.
  - touches: `website/agent-detection/SUBMITTING.md`
  - depends on: 2.1

- [ ] 2.3 Add a focused agent-detection pull-request template that requires representative evidence, manifest-check output, fixture coverage, and explicit matcher-content review. Expected result: every accepted submission exposes both automated and human trust gates.
  - touches: `.github/PULL_REQUEST_TEMPLATE/agent-detection.md`
  - depends on: 2.2

## 3. Verify the contribution contract

- [ ] 3.1 Run the focused manifest fixture tests, `python3 scripts/agent_detection_manifest_check.py`, and `just check`. Expected result: the evidence path, bundled/catalog invariants, and repository-wide validation pass before archiving this change.
  - touches: none (validation only)
  - depends on: 2.3
