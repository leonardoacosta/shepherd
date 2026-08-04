## 1. Characterize prerequisites

- [ ] 1.1 Add tests for overlay exit restoration, config path resolution, valid/invalid live reload, and unchanged files; run `just test-one config_io` and the focused overlay tests.
- [ ] 1.2 Add platform contract tests for editor precedence, argv boundaries, and fallbacks on Linux, macOS, and Windows; run the platform-focused test filters.

## 2. Implement server-owned editing

- [ ] 2.1 Add neutral API request/result/event types and single-flight operation state in `src/api/schema`, `src/app/api`, `src/app/mod.rs`, and `src/events.rs`; run generated schema tests and expect stable duplicate/unavailable behavior.
- [ ] 2.2 Add dedicated platform editor argv helpers and config-path parent handling in `src/platform/{linux,macos,windows,fallback}.rs` and config I/O; run platform tests and expect one positional path argument.
- [ ] 2.3 Capture editor exit status, compare pre/post bytes, and route valid/invalid outcomes through the existing reload pipeline without temp-file cleanup; run `just test-one config_editor` and expect all outcome cases to pass.

## 3. Add presentation and consumers

- [ ] 3.1 Add the Settings advanced action and bounded diagnostics while keeping overlay presentation client-local; run `just test-one settings` and expect keyboard/mouse reachability at supported sizes.
- [ ] 3.2 Update generated schema/TypeScript consumers and next configuration/API documentation; run `python3 -m unittest scripts.test_socket_api_reference_check && bun --cwd clients/ts test` and expect parity.

## 4. Verify and hand off

- [ ] 4.1 Run `just check` and expect formatting, tests, generated assets, platform checks, and maintenance suites to pass.
- [ ] 4.2 Run `openspec validate open-advanced-config-editor --strict --no-interactive && git diff --check`; expect both commands to exit 0.
