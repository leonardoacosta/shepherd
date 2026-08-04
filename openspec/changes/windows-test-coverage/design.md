## Context

`scripts/windows_check.ps1` currently builds the `shepherd` unit-test binary but selects only `windows_` tests and `server::client_transport::tests`. That makes the gate reliable but leaves most cross-platform behavior unexecuted on Windows. Seven modules also use module-wide `#[cfg(all(test, unix))]` gates, which can hide portable tests alongside genuinely Unix-specific cases.

This work must be measured on the project-maintained Windows VM. Cross-compilation and Windows-target Clippy provide compile evidence, but they cannot establish which tests pass under ConPTY, Windows filesystem semantics, or Windows process behavior.

## Goals / Non-Goals

**Goals:**

- Establish the complete Windows unit-suite result before editing the gate.
- Make every excluded test visible with a concise, reviewable reason.
- Run portable tests on Windows even when adjacent tests remain Unix-only.
- Keep the Windows job within a practical CI budget by sharding if needed.

**Non-Goals:**

- Running `tests/*.rs` integration binaries on Windows.
- Hiding product defects by permanently quarantining them without a follow-up owner.
- Adding a new test-runner dependency solely for this change.

## Decisions

### Measure with the full unit-test binary first

Use the Windows VM to run the complete `shepherd` binary unit suite before selecting exclusions. The implementation may use Cargo's existing test runner because it is already installed and exercised by `windows_check.ps1`; a one-off nextest measurement is acceptable when available but is not a new CI dependency.

### Represent quarantine as explicit policy

The final Windows invocation runs the broad suite minus a small, documented list of exact exclusions. Every exclusion records whether it is Windows-inapplicable, a tracked product defect, or a demonstrated flake. Broad substring filters such as `windows_` are not an acceptable representation of the passing suite.

### Narrow platform gates around actual dependencies

For modules currently hidden wholesale on Windows, move Unix gates to imports, helpers, or individual tests when the remaining test logic is portable. Tests that exercise Unix sockets, signals, permissions, or other unavailable APIs stay compile-gated with the reason evident from the gated dependency.

### Shard before reducing coverage

If the complete passing suite exceeds the Windows CI budget, split it into deterministic shards. A timeout is evidence to change scheduling, not evidence to restore the narrow name filter.

## Risks / Trade-offs

- **The first full run may expose many failures** → Classify results before code changes and create separate owners for real product defects.
- **Quarantine can become permanent** → Keep exclusions exact and reasoned so later work can remove them individually.
- **Windows runtime may grow materially** → Use deterministic shards while preserving the same aggregate coverage.
- **Regating can accidentally compile Unix dependencies on Windows** → Gate the smallest platform-specific import/helper and validate in the Windows VM after each affected module.

## Migration Plan

1. Sync the approved change to `C:\work\repo` and measure the unfiltered unit binary.
2. Record pass, failure, and timeout evidence; establish the initial quarantine.
3. Re-gate portable tests and rerun the full measurement.
4. Update `windows_check.ps1` to execute the measured passing suite, sharded if needed.
5. Validate on Windows, restore the VM checkout to clean state, then run the repository-wide Linux checks.

## Open Questions

- The exact quarantine and shard boundaries cannot be chosen until the authoritative Windows measurement is captured.
