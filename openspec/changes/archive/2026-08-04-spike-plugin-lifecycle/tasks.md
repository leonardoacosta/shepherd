## 1. Preserve plugin registry compatibility

- [x] 1.1 Confirm the existing source provenance and version fields cover upstream comparison, preserve local defaults for linked plugins, and add a regression proving source-less legacy registry JSON deserializes. Run `just test-one plugin_registry`; expected result: the legacy entry loads without a migration and behaves as local provenance.
  - touches: `src/persist/plugin_registry.rs`
  - depends on: []

## 2. Deliver manual upstream drift reporting

- [x] 2.1 Implement `shepherd plugin outdated [--plugin ID] [--json]` with current, outdated, pinned, `n/a`, and unknown results; use at most one `git ls-remote` query for each comparable GitHub plugin and isolate failures per plugin. Run `just test-one plugin`; expected result: ref matching, moved refs, annotated tags, pins, local entries, missing provenance, ambiguity, failures, filtering, and output behavior pass.
  - touches: `src/cli/spec.rs`, `src/cli/plugin.rs`
  - depends on: 1.1

- [x] 2.2 Record and accept the conservative lifecycle decision: keep drift reporting manual and read-only, with no automatic update, background polling, code fetch, execution, API state, or TUI surface. Expected result: `docs/next/plugin-lifecycle-spike.md` identifies the decision as accepted and names separate-proposal prerequisites for future expansion.
  - touches: `docs/next/plugin-lifecycle-spike.md`
  - depends on: 2.1

## 3. Verify and archive

- [x] 3.1 Run `just test-one plugin_registry`, `just test-one plugin`, and `just check`; expected result: migration, drift-reporting, formatting, test, maintenance, and cross-target checks pass on the final bytes before normal OpenSpec archive.
  - touches: none (validation only)
  - depends on: 2.2
