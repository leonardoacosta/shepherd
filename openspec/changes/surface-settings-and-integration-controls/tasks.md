## 1. Protect the broad-change baseline

- [ ] 1.1 Refresh the base/dirty baseline, active OpenSpec touch claims, integration target inventory, latest released protocol versus `PROTOCOL_VERSION`, and relevant config bounds; update proposal/design only for discovered drift. Run `openspec list --json && git status --short && git diff --check`, then inspect `git show "$(git tag --sort=-version:refname | rg '^v[0-9]+\.[0-9]+\.[0-9]+$' | head -1)":src/protocol/wire.rs` beside current `src/protocol/wire.rs`; expected result: the implementation starts from declared, non-overlapping claims, preserves unrelated `improvements.md`, and records the release-relative protocol decision.
  - touches: `openspec/changes/surface-settings-and-integration-controls/proposal.md`, `openspec/changes/surface-settings-and-integration-controls/design.md`
  - depends on: []

- [ ] 1.2 Complete the project-required broad-change roundtable over Settings geometry/input, terminal authority/API projection, config persistence, and auxiliary-pane identity; name characterization tests using `AppState::assert_invariants_for_test()` or `Workspace::assert_invariants_for_test()` with adversarial identity state. Expected result: accepted risks and protected behaviors are recorded in `design.md` before structural edits.
  - touches: `openspec/changes/surface-settings-and-integration-controls/design.md`
  - depends on: 1.1

## 2. Establish responsive Settings geometry

- [ ] 2.1 Add failing pure geometry/render/input tests for the ordered Theme/Sound/Toast/Display/Behavior/Integrations/Experiments sections, overflow, badges, stable row identities, scrolling, resize normalization, and mouse parity at 40x20, 64x20, and 80x24. Run `just test-one settings`; expected result: the new cases expose current clipping/hit-target failures while existing Settings behavior remains characterized.
  - touches: `src/ui/settings.rs`, `src/app/state.rs`, `src/app/input/settings.rs`
  - depends on: 1.2

- [ ] 2.2 Implement the pure Settings view model, adaptive section viewport, typed visible rows, per-section offsets, and shared scrollbar geometry; make rendering and keyboard/mouse input consume that model. Run `just test-one settings`; expected result: all width, resize, dynamic-row, scroll, badge, and parity cases pass without render mutation.
  - touches: `src/ui/settings.rs`, `src/ui/scrollbar.rs`, `src/app/state.rs`, `src/app/input/settings.rs`, `src/app/input/mouse.rs`
  - depends on: 2.1

## 3. Expose neutral agent state source

- [ ] 3.1 Add failing terminal/API cases for screen, mixed report, exclusive lifecycle report, clear/expiry fallback, restored session-only identity, and PaneInfo/AgentInfo parity. Run `just test-one agent_state_source`; expected result: cases fail because the typed projection does not yet exist while current authority behavior stays characterized.
  - touches: `src/terminal/state.rs`, `src/app/agents.rs`, `src/api/schema/agents.rs`, `src/api/schema/panes.rs`, `src/api/schema/tests.rs`
  - depends on: 1.2

- [ ] 3.2 Implement `AgentStateSource`/`AgentReportAuthority` and one terminal projection used by PaneInfo and AgentInfo; retain `screen_detection_skipped` as an exact derived compatibility value and keep session identity separate. Run `just test-one agent_state_source`; expected result: every source/parity/fallback case passes with no timestamp-based health classification.
  - touches: `src/terminal/state.rs`, `src/app/agents.rs`, `src/api/schema/agents.rs`, `src/api/schema/panes.rs`, `src/api/schema/tests.rs`
  - depends on: 3.1

- [ ] 3.3 Regenerate the public JSON schema and TypeScript client for the optional typed source fields, compare protocol version to the latest release before any bump, and update exact fixtures only when required. Run `python3 -m unittest scripts.test_socket_api_reference_check && bun --cwd clients/ts test`; expected result: schema/client types and compatibility fixtures agree with the Rust contract.
  - touches: `src/api/schema.rs`, `src/protocol/wire.rs`, `docs/next/api/shepherd-api.schema.json`, `clients/ts/src/index.ts`, `clients/ts/test/client.test.ts`, `clients/ts/test/generate-types.test.ts`, `scripts/test_socket_api_reference_check.py`
  - depends on: 3.2

## 4. Build the integration manager

- [ ] 4.1 Preserve support/availability, path, current version, and expected version in the integration status projection; add target-keyed row/result state and failing small-viewport/partial-failure tests. Run `just test-one integration && just test-one settings`; expected result: richer data tests pass while action/reachability cases fail before UI wiring.
  - touches: `src/integration/types.rs`, `src/integration/registry.rs`, `src/app/state.rs`, `src/ui/settings.rs`, `src/app/input/settings.rs`
  - depends on: 2.2, 3.3

- [ ] 4.2 Implement per-target install/update actions, globally single-flight integration mutation state, refresh, disabled explanations, and keyboard/mouse target parity through existing neutral integration APIs; keep eligible bulk install secondary and serial. Run `just test-one integration && just test-one settings`; expected result: only selected/eligible targets mutate in registry order, statuses refresh, races are excluded, and all rows remain reachable at required sizes.
  - touches: `src/app/api/integrations.rs`, `src/app/api.rs`, `src/app/state.rs`, `src/app/input/settings.rs`, `src/ui/settings.rs`, `src/api/schema/integrations.rs`
  - depends on: 4.1

- [ ] 4.3 Add explicit target/path uninstall confirmation plus complete target-keyed operation history with bounded summary and scrollable detail. Run `just test-one settings && just test-one integration`; expected result: cancel is mutation-free and a failure after multiple successes is visible/reachable at 40x20.
  - touches: `src/app/state.rs`, `src/app/input/settings.rs`, `src/app/input/mod.rs`, `src/ui/settings.rs`, `src/app/api/integrations.rs`
  - depends on: 4.2

- [ ] 4.4 Aggregate typed state-source and session-source facts into descriptive per-integration counts using exact canonical target-source matches without identifiers, health colors, freshness claims, or guessed unknown-source attribution. Run `just test-one integration_observation`; expected result: lifecycle, mixed, session-only, persisted-session, absent-observation, and unknown-source cases are truthful and source counts match current panes.
  - touches: `src/app/state.rs`, `src/ui/settings.rs`, `src/app/agents.rs`, `src/integration/types.rs`
  - depends on: 4.3

## 5. Surface everyday preferences

- [ ] 5.1 Add typed Display/Behavior row definitions and failing 40/64/80 keyboard/mouse tests for pane borders/gaps, hide-single-tab-bar, close confirmation, naming prompts, copy-on-select, scroll lines, Agent sort bounds, and the terminal `edit config.toml` action row. Run `just test-one settings`; expected result: the intended row and parity cases fail before controls are wired.
  - touches: `src/app/state.rs`, `src/ui/settings.rs`, `src/app/input/settings.rs`, `src/config/model.rs`
  - depends on: 2.2

- [ ] 5.2 Add dedicated one-key comment-preserving writers, wire controls to live reload, retain last valid values on malformed/write failure, and label any proven next-launch-only behavior. Run `just test-one config_io && just test-one settings`; expected result: each key changes alone, comments survive, bounds hold, live consumers update, and failures do not create state/config divergence.
  - touches: `src/app/config_io.rs`, `src/config/io.rs`, `src/app/input/settings.rs`, `src/app/mod.rs`, `src/ui/settings.rs`
  - depends on: 5.1

- [ ] 5.3 Clarify the Agent header as `sort: grouped`/`sort: priority` and prove sidebar/Settings controls persist and reflect the same `ui.agent_panel_sort` value. Run `just test-one sidebar && just test-one settings`; expected result: both input paths stay synchronized with no duplicate persisted state.
  - touches: `src/config/sidebar.rs`, `src/ui/sidebar.rs`, `src/app/input/sidebar.rs`, `src/app/input/mod.rs`, `src/app/config_io.rs`, `src/app/mod.rs`
  - depends on: 5.2

## 6. Add the advanced editor bridge

- [ ] 6.1 Add failing API/platform/lifecycle tests for `server.config.edit`, no-surface failure, safe argv/path boundaries, platform fallbacks, single-flight duplicate calls, disconnect survival, auxiliary-pane navigation/snapshot exclusion, file digest/existence transitions, external final-byte authority, and stable identity on exit. Run `just test-one config_editor`; expected result: contract cases fail before the neutral operation exists while existing overlay characterization remains green.
  - touches: `src/api/schema.rs`, `src/api/schema/server.rs`, `src/api/schema/events.rs`, `src/api/schema/tests.rs`, `src/app/api.rs`, `src/app/input/navigate.rs`, `src/platform/mod.rs`, `src/platform/linux.rs`, `src/platform/macos.rs`, `src/platform/windows.rs`, `src/platform/fallback.rs`
  - depends on: 1.2, 5.2

- [ ] 6.2 Implement platform-isolated config editor argv, server path resolution, the public start response, single-flight server-owned auxiliary pane, TUI overlay presentation, and no-temp-cleanup lifecycle. Run `just test-one config_editor`; expected result: local and remote-capable calls return stable pane identity, duplicates reuse it, paths remain single arguments, and Shepherd never deletes the config file.
  - touches: `src/platform/mod.rs`, `src/platform/linux.rs`, `src/platform/macos.rs`, `src/platform/windows.rs`, `src/platform/fallback.rs`, `src/app/input/navigate.rs`, `src/app/api.rs`, `src/app/runtime_mutations.rs`, `src/server/headless.rs`, `src/api/schema.rs`, `src/api/schema/server.rs`
  - depends on: 6.1

- [ ] 6.3 Implement digest-aware exit validation/reload outcomes, valid-save precedence over nonzero editor status, deliberate removal/default reload, final external-byte authority, last-valid runtime preservation, retained unreadable/invalid bytes, actionable diagnostics, `server.config_edit_finished`, and immediate Settings reopen feedback. Run `just test-one config_editor && just test-one config_io`; expected result: reloaded/unchanged/invalid/editor_failed cases emit once and invalid/unreadable TOML never replaces runtime state.
  - touches: `src/app/api.rs`, `src/app/config_io.rs`, `src/app/input/settings.rs`, `src/app/state.rs`, `src/ui/settings.rs`, `src/api/schema/events.rs`, `src/api/schema/response.rs`, `src/api/schema/tests.rs`, `src/server/headless.rs`
  - depends on: 6.2

- [ ] 6.4 Regenerate schema/client artifacts for the editor method, response, subscription, and event; update protocol expectations only under the release-relative rule. Run `python3 -m unittest scripts.test_socket_api_reference_check && bun --cwd clients/ts test`; expected result: Rust, JSON schema, TypeScript types, event lists, and protocol fixtures agree.
  - touches: `src/api/schema.rs`, `src/api/schema/events.rs`, `src/api/schema/response.rs`, `src/protocol/wire.rs`, `docs/next/api/shepherd-api.schema.json`, `clients/ts/src/index.ts`, `clients/ts/test/client.test.ts`, `clients/ts/test/generate-types.test.ts`, `scripts/test_socket_api_reference_check.py`
  - depends on: 6.3

## 7. Document and verify the complete program

- [ ] 7.1 Update next configuration docs/locales, generated config reference, and unreleased changelog for responsive Settings, target management, descriptive observations, curated preferences, and advanced editing. Run `python3 -m unittest scripts.test_config_reference_check scripts.test_docs_translation_parity`; expected result: reference and locale parity pass without stable-doc changes.
  - touches: `docs/next/CHANGELOG.md`, `docs/next/website/src/content/docs/configuration.mdx`, `docs/next/website/src/content/docs/ja/configuration.mdx`, `docs/next/website/src/content/docs/zh-cn/configuration.mdx`, `docs/next/website/src/data/config-reference.json`, `scripts/test_config_reference_check.py`
  - depends on: 4.4, 5.3, 6.4

- [ ] 7.2 Run `just test-one settings && just test-one integration && just test-one agent_state_source && just test-one config_editor && just test-one config_io && just test-one sidebar`; expected result: responsive geometry, target management, neutral observation, preferences, editor lifecycle, and adversarial identity invariants all pass.
  - touches: none (read-only validation)
  - depends on: 7.1

- [ ] 7.3 Run `openspec validate surface-settings-and-integration-controls --strict --no-interactive && git diff --check && git diff --exit-code -- Cargo.toml Cargo.lock README.md CHANGELOG.md website/src/content/docs website/latest.json`; expected result: strict/whitespace checks exit 0 and there is no dependency, root release-doc, stable-doc, or release-channel diff.
  - touches: none (read-only validation)
  - depends on: 7.2

- [ ] 7.4 Run `just check`; expected result: formatting, nextest, Windows-target lint, integration assets, generated clients/docs, and maintenance suites all exit 0; final status and active-claim checks show only declared paths plus apply-owned OpenSpec changes.
  - touches: none (read-only validation)
  - depends on: 7.3
