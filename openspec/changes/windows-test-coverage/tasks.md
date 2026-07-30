# Tasks — windows-test-coverage

Base commit: `1de05dc2`. Drift check: confirm `scripts/windows_check.ps1:50-68`
still filters `cargo test` to `windows_` + `server::client_transport::tests`. If
the Windows CI already runs a broader suite, STOP and report.

Exemplar: `scripts/windows_check.ps1` current `Invoke-Checked cargo @(...)` calls
— the invocation shape to extend. This work needs a Windows environment; per
AGENTS.md the Windows VM (`windows-wirt` alias, `C:\work\repo`) is the validation
target for maintainers.

## Ordered steps

1. **Measure.** On a Windows target, run
   `cargo nextest run --target x86_64-pc-windows-msvc --bin herdr` unfiltered.
   Capture the full pass/fail/timeout list.

2. **Triage.** Split failures into: (a) genuinely Windows-inapplicable, (b) real
   Windows bugs (file separately as issues/proposals), (c) flaky. Build a pass
   list and a quarantine list with a one-line reason per quarantined test.

3. **Re-gate portable tests.** For the seven `#[cfg(all(test, unix))]` modules,
   identify tests with no unix-specific dependency and change their gating to
   per-test (or unconditional) so they compile+run on Windows.

4. **Update the CI gate.** Change `windows_check.ps1` to run the pass list
   (e.g. via a nextest filter expression or a partition) instead of only
   `windows_`. Keep the quarantine excluded but documented.
   - Gate: Windows CI job green on the new pass list.

5. **Verify and close.**
   - `just check` still green (unix + windows-target clippy). done-when: proposal
     `windows-test-coverage` archived.

## STOP conditions

- If the unfiltered run cannot complete within a reasonable CI budget even
  sharded, report the numbers and the maintainer decides the coverage/time
  tradeoff before gating.
