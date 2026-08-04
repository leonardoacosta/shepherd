## 1. Characterize current authority

- [ ] 1.1 Add terminal tests for screen evidence, exclusive lifecycle evidence, mixed evidence, screen-overridden mixed evidence, clear/expiry, and persisted-session-only cases; run `just test-one agent_state_source` and expect all arbitration cases to pass.
- [ ] 1.2 Add a parity characterization proving PaneInfo and AgentInfo use the same terminal projection; run `just test-one agents` and expect identical evidence values.

## 2. Implement the typed projection

- [ ] 2.1 Add shared state-evidence schema types and terminal projection logic in `src/terminal/state.rs`, `src/api/schema/agents.rs`, and `src/api/schema/panes.rs`; run focused Rust tests and expect screen/reported authority values to serialize deterministically.
- [ ] 2.2 Wire `src/app/creation.rs` and `src/app/agents.rs` to the shared helper and derive `screen_detection_skipped` from effective exclusive evidence; run `just test-one agent_state_source` and expect parity and compatibility tests to pass.

## 3. Add client observations

- [ ] 3.1 Add exact-source reporter/session aggregation to the integration view model without exposing identifiers or health colors; run `just test-one integration` and expect canonical, absent, and unknown-source cases to pass.
- [ ] 3.2 Update generated schema, TypeScript client tests, and next API documentation; run `python3 -m unittest scripts.test_socket_api_reference_check && bun --cwd clients/ts test` and expect generated contracts to match.

## 4. Verify and hand off

- [ ] 4.1 Compare `src/protocol/wire.rs::PROTOCOL_VERSION` with the latest release, update fixtures only if required, then run `just check` and expect all repository checks to pass.
- [ ] 4.2 Run `openspec validate expose-agent-state-source --strict --no-interactive && git diff --check`; expect both commands to exit 0.
