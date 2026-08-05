## 1. Protect the broad-change baseline

- [x] 1.1 Refresh the base/dirty baseline, active OpenSpec touch claims, integration target inventory, latest released protocol versus `PROTOCOL_VERSION`, and relevant config bounds; update proposal/design only for discovered drift. Run `openspec list --json && git status --short && git diff --check`, then inspect `git show "$(git tag --sort=-version:refname | rg '^v[0-9]+\.[0-9]+\.[0-9]+$' | head -1)":src/protocol/wire.rs` beside current `src/protocol/wire.rs`; expected result: the implementation starts from declared, non-overlapping claims, preserves unrelated `improvements.md`, and records the release-relative protocol decision.
  - touches: `openspec/changes/surface-settings-and-integration-controls/proposal.md`, `openspec/changes/surface-settings-and-integration-controls/design.md`
  - depends on: []

- [x] 1.2 Complete the project-required broad-change roundtable over Settings geometry/input and config persistence; name characterization tests using `AppState::assert_invariants_for_test()` or `Workspace::assert_invariants_for_test()` with adversarial identity state. Expected result: accepted risks and protected behaviors are recorded in `design.md` before structural edits.
  - touches: `openspec/changes/surface-settings-and-integration-controls/design.md`
  - depends on: 1.1

## 2. Establish responsive Settings geometry

- [x] 2.1 Add failing pure geometry/render/input tests for the ordered Theme/Sound/Toast/Display/Behavior/Integrations/Experiments sections, overflow, badges, stable row identities, scrolling, resize normalization, and mouse parity at 40x20, 64x20, and 80x24. Run `just test-one settings`; expected result: the new cases expose current clipping/hit-target failures while existing Settings behavior remains characterized.
  - touches: `src/ui/settings.rs`, `src/app/state.rs`, `src/app/input/settings.rs`
  - depends on: 1.2

- [x] 2.2 Implement the pure Settings view model, adaptive section viewport, typed visible rows, per-section offsets, and shared scrollbar geometry; make rendering and keyboard/mouse input consume that model. Run `just test-one settings`; expected result: all width, resize, dynamic-row, scroll, badge, and parity cases pass without render mutation.
  - touches: `src/ui/settings.rs`, `src/ui/scrollbar.rs`, `src/app/state.rs`, `src/app/input/settings.rs`, `src/app/input/mouse.rs`
  - depends on: 2.1

## 3. Build the integration manager

- [x] 3.1 Preserve support/availability, path, current version, and expected version in the integration status projection; add target-keyed row/result state and failing small-viewport/partial-failure tests. Run `just test-one integration && just test-one settings`; expected result: richer data tests pass while action/reachability cases fail before UI wiring.
  - touches: `src/integration/types.rs`, `src/integration/registry.rs`, `src/app/state.rs`, `src/ui/settings.rs`, `src/app/input/settings.rs`
  - depends on: 2.2

- [x] 3.2 Implement per-target install/update actions, globally single-flight integration mutation state, refresh, disabled explanations, and keyboard/mouse target parity through existing neutral integration APIs; keep eligible bulk install secondary and serial. Run `just test-one integration && just test-one settings`; expected result: only selected/eligible targets mutate in registry order, statuses refresh, races are excluded, and all rows remain reachable at required sizes.
  - touches: `src/app/api/integrations.rs`, `src/app/api.rs`, `src/app/state.rs`, `src/app/input/settings.rs`, `src/ui/settings.rs`, `src/api/schema/integrations.rs`
  - depends on: 3.1

- [x] 3.3 Add explicit target/path uninstall confirmation plus complete target-keyed operation history with bounded summary and scrollable detail. Run `just test-one settings && just test-one integration`; expected result: cancel is mutation-free and a failure after multiple successes is visible/reachable at 40x20.
  - touches: `src/app/state.rs`, `src/app/input/settings.rs`, `src/app/input/mod.rs`, `src/ui/settings.rs`, `src/app/api/integrations.rs`
  - depends on: 3.2

## 4. Surface everyday preferences

- [x] 4.1 Add typed Display/Behavior row definitions and failing 40/64/80 keyboard/mouse tests for pane borders/gaps, hide-single-tab-bar, close confirmation, naming prompts, copy-on-select, scroll lines, and Agent sort bounds. Run `just test-one settings`; expected result: the intended row and parity cases fail before controls are wired.
  - touches: `src/app/state.rs`, `src/ui/settings.rs`, `src/app/input/settings.rs`, `src/config/model.rs`
  - depends on: 2.2

- [x] 4.2 Add dedicated one-key comment-preserving writers, wire controls to live reload, retain last valid values on malformed/write failure, and label any proven next-launch-only behavior. Run `just test-one config_io && just test-one settings`; expected result: each key changes alone, comments survive, bounds hold, live consumers update, and failures do not create state/config divergence.
  - touches: `src/app/config_io.rs`, `src/config/io.rs`, `src/app/input/settings.rs`, `src/app/mod.rs`, `src/ui/settings.rs`
  - depends on: 4.1

- [x] 4.3 Clarify the Agent header as `sort: grouped`/`sort: priority` and prove sidebar/Settings controls persist and reflect the same `ui.agent_panel_sort` value. Run `just test-one sidebar && just test-one settings`; expected result: both input paths stay synchronized with no duplicate persisted state.
  - touches: `src/config/sidebar.rs`, `src/ui/sidebar.rs`, `src/app/input/sidebar.rs`, `src/app/input/mod.rs`, `src/app/config_io.rs`, `src/app/mod.rs`
  - depends on: 4.2

## 5. Document and verify the complete program

- [x] 5.1 Update next configuration docs/locales, generated config reference, and unreleased changelog for responsive Settings, target management, and curated preferences. Run `python3 -m unittest scripts.test_config_reference_check scripts.test_docs_translation_parity`; expected result: reference and locale parity pass without stable-doc changes.
  - touches: `docs/next/CHANGELOG.md`, `docs/next/website/src/content/docs/configuration.mdx`, `docs/next/website/src/content/docs/ja/configuration.mdx`, `docs/next/website/src/content/docs/zh-cn/configuration.mdx`, `docs/next/website/src/data/config-reference.json`, `scripts/test_config_reference_check.py`
  - depends on: 3.3, 4.3

- [x] 5.2 Run `just test-one settings && just test-one integration && just test-one config_io && just test-one sidebar`; expected result: responsive geometry, target management, preferences, and adversarial identity invariants all pass.
  - touches: none (read-only validation)
  - depends on: 5.1

- [x] 5.3 Run `openspec validate surface-settings-and-integration-controls --strict --no-interactive && git diff --check && git diff --exit-code -- Cargo.toml Cargo.lock README.md CHANGELOG.md website/src/content/docs website/latest.json`; expected result: strict/whitespace checks exit 0 and there is no dependency, root release-doc, stable-doc, or release-channel diff.
  - touches: none (read-only validation)
  - depends on: 5.2

- [x] 5.4 Run `just check`; expected result: formatting, nextest, Windows-target lint, integration assets, generated clients/docs, and maintenance suites all exit 0; final status and active-claim checks show only declared paths plus apply-owned OpenSpec changes.
  - touches: none (read-only validation)
  - depends on: 5.3
