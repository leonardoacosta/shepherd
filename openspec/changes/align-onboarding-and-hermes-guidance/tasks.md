## 1. Characterize authoritative guidance

- [ ] 1.1 Add focused characterization cases for default/custom/unset/multiple action labels and current 40-column onboarding cells before changing rendering. Run `just test-one onboarding`; expected result: the new cases fail only where hard-coded or clipped guidance differs from the specification.
  - touches: `src/ui/onboarding.rs`, `src/ui/keybind_help.rs`
  - depends on: []

- [ ] 1.2 Reconfirm Hermes session-only authority and the bundled version against current source/tests, and record any source drift in the OpenSpec design before editing docs. Run `just test-one hermes`; expected result: terminal/detection and asset tests prove session identity plus screen-derived state and print/pass the current version contract.
  - touches: `openspec/changes/align-onboarding-and-hermes-guidance/design.md`
  - depends on: []

## 2. Render effective onboarding shortcuts

- [ ] 2.1 Share the existing human-readable action-label helper and derive onboarding prefix/help/Settings labels from effective `AppState` keybindings. Run `just test-one onboarding`; expected result: default, customized, unset, and multiple-binding cases render the same labels as the help surface.
  - touches: `src/ui/keybind_help.rs`, `src/ui/onboarding.rs`
  - depends on: 1.1

- [ ] 2.2 Replace the one-line literal instruction with a measured semantic shortcut block that preserves every label at 40 columns and remains render-pure. Run `just test-one onboarding`; expected result: narrow/normal buffers contain complete labels without overlap and repeated renders do not mutate state.
  - touches: `src/ui/onboarding.rs`
  - depends on: 2.1

## 3. Correct unreleased Hermes documentation

- [ ] 3.1 Update English, Japanese, and Chinese next-version agent/integration pages to describe Hermes session identity, screen-manifest state, and the current bundled version; strengthen narrow parity checks without touching stable docs. Run `python3 -m unittest scripts.test_docs_translation_parity scripts.test_hermes_integration_asset`; expected result: all locales and asset/version assertions pass.
  - touches: `docs/next/website/src/content/docs/integrations.mdx`, `docs/next/website/src/content/docs/agents.mdx`, `docs/next/website/src/content/docs/ja/integrations.mdx`, `docs/next/website/src/content/docs/ja/agents.mdx`, `docs/next/website/src/content/docs/zh-cn/integrations.mdx`, `docs/next/website/src/content/docs/zh-cn/agents.mdx`, `scripts/test_docs_translation_parity.py`
  - depends on: 1.2

## 4. Verify the feature

- [ ] 4.1 Run `just test-one onboarding && python3 -m unittest scripts.test_docs_translation_parity scripts.test_hermes_integration_asset`; expected result: every focused UI and Hermes guidance contract passes.
  - touches: none (read-only validation)
  - depends on: 2.2, 3.1

- [ ] 4.2 Run `openspec validate align-onboarding-and-hermes-guidance --strict --no-interactive && git diff --check && git diff --exit-code -- src/detect src/integration website/src/content/docs`; expected result: validation and whitespace checks exit 0 and no runtime, integration-asset, or stable-doc diff exists.
  - touches: none (read-only validation)
  - depends on: 4.1

- [ ] 4.3 Run `just check`; expected result: formatting, tests, generated assets, docs, and maintenance checks all exit 0.
  - touches: none (read-only validation)
  - depends on: 4.2
