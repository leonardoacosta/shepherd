# Run the full unit suite on Windows instead of a name-filtered slice

Base commit: `1de05dc2` · Route: proposal · Effort: S to measure, M to triage · Confidence: HIGH · Category: test coverage

## Why

Resolves advisory finding TESTS-01 (audit against `1de05dc2`).

Windows is a shipped beta (own install script, docs page, ConPTY packaging) but
the shared codebase is only *compiled* on Windows, never executed under test:

- `scripts/windows_check.ps1:50-68` — Windows `check` mode runs only
  `cargo test --bin shepherd windows_` plus `server::client_transport::tests`. That
  is ~75 `windows_*` tests plus one module, against ~3,138 `#[test]`/`#[tokio::test]`
  sites repo-wide.
- `.github/workflows/ci.yml:144` — the windows-latest job invokes only that
  script; no full `cargo nextest`, no integration binaries.
- Seven modules gate their tests `#[cfg(all(test, unix))]` (`src/api/server.rs`,
  `src/pane.rs`, `src/platform/mod.rs`, `src/server/socket_paths.rs`,
  `src/server/autodetect.rs`, `src/client/input.rs`, `src/update.rs`), so their
  tests do not even compile on Windows.

The test binary is already built by that `cargo test` invocation — the tests are
being deliberately filtered out, not blocked. Recent history shows the cost:
`fix: make native windows checks reliable`, `fix: preserve shift-enter in
windows panes (#1909)`.

## What changes

1. Run `cargo nextest run --target x86_64-pc-windows-msvc --bin shepherd` unfiltered
   once (locally or a scratch CI run) and triage results into a pass list and a
   quarantine list.
2. Make the pass list the Windows CI gate; shrink the quarantine over time.
3. Audit the seven `#[cfg(all(test, unix))]` modules for genuinely
   platform-portable tests and re-gate those per-test rather than per-module.

## Acceptance

- Windows CI executes materially more than the current `windows_*` slice — the
  new pass list — and is green.
- The quarantine list (tests that genuinely fail/flake on Windows) is recorded
  with a reason per entry, not silently dropped.
- `just check` (which already runs windows-target clippy) still passes.

## Out of scope

- Fixing every Windows test failure the unfiltered run surfaces — triage first;
  real Windows bugs found this way become their own issues/proposals.
- Running the integration binaries (`tests/*.rs`) on Windows — that is a larger
  step; this change is scoped to the in-`shepherd`-binary unit suite.

## STOP / judgment

- If the unfiltered run blows the CI job timeout, split into shards rather than
  re-filtering back down to `windows_*`.
