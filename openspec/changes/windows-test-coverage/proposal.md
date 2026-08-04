## Why

Windows is a shipped beta, but its CI check runs only tests whose names contain `windows_` plus `server::client_transport::tests`. The same unit-test binary contains materially broader portable behavior, while several modules still gate their entire test modules to Unix. Shepherd needs measured Windows execution coverage rather than a permanently name-filtered sample.

## What Changes

- Measure the complete `shepherd` binary unit suite on the maintained Windows VM before choosing exclusions.
- Classify every failure as a genuine Windows incompatibility, a product defect, or a flake, with an explicit reason for each temporary quarantine.
- Re-gate platform-portable tests currently hidden behind module-wide Unix gates so they compile and run on Windows.
- Replace the narrow Windows CI filters with the measured passing suite minus the documented quarantine, sharding when necessary instead of returning to name filtering.

## Capabilities

### New Capabilities

- `windows-unit-test-gate`: Measured, broad Windows unit-test execution with explicit quarantines and portable-test coverage.

### Modified Capabilities

None.

## Impact

- Windows validation: `scripts/windows_check.ps1` and the existing Windows CI job will execute materially more unit tests after measurement and triage.
- Test gating: portable tests in modules currently guarded by `#[cfg(all(test, unix))]` may move to narrower per-test or dependency-level gates.
- Planning boundary: this change does not require Windows integration binaries, fixing every defect found during measurement, or replacing the existing Cargo test runner.
- Validation environment: the maintained Windows VM at `C:\work\repo` is the authoritative measurement and manual validation target.
