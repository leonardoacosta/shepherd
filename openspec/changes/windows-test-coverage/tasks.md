## 1. Measure the current Windows unit boundary

- [ ] 1.1 Sync the approved tree to the maintained Windows VM checkout at `C:\work\repo`, run the complete `shepherd` binary unit suite without the current name filters, and capture exact pass, fail, and timeout evidence. Run `cargo test --target x86_64-pc-windows-msvc --bin shepherd -- --list` followed by the unfiltered binary test command; expected result: the complete inventory and runtime outcome are recorded before gate edits.
  - touches: none (Windows measurement only)
  - depends on: []

- [ ] 1.2 Classify every observed failure as Windows-inapplicable, a product defect, or a demonstrated flake, and record an exact test name plus one-line reason and follow-up owner for each quarantine candidate. Expected result: no broad module or substring exclusion is proposed without itemized evidence.
  - touches: planning notes under `.local/` or the eventual quarantine comments in `scripts/windows_check.ps1`
  - depends on: 1.1

## 2. Expand portable Windows execution

- [ ] 2.1 Audit the modules still using `#[cfg(all(test, unix))]`, move platform gates to the narrowest imports, helpers, or individual tests, and rerun the affected tests on Windows. Expected result: portable tests compile and run on Windows while genuinely Unix-only tests remain gated.
  - touches: `src/api/server.rs`, `src/pane.rs`, `src/platform/mod.rs`, `src/server/socket_paths.rs`, `src/server/autodetect.rs`, `src/client/input.rs`, `src/update.rs`
  - depends on: 1.2

- [ ] 2.2 Replace the narrow filters in `scripts/windows_check.ps1` with the measured broad passing suite minus exact reasoned quarantines; use deterministic shards if the full gate exceeds the job budget. Run the script's `check` mode on the Windows VM; expected result: materially more unit tests execute and every exclusion is visible and explained.
  - touches: `scripts/windows_check.ps1`
  - depends on: 1.2, 2.1

## 3. Verify the gate

- [ ] 3.1 Run the updated Windows check in `C:\work\repo`, confirm the Windows CI job shape remains compatible, restore the VM checkout to clean state, and run `just check` on Linux. Expected result: the broad Windows gate and repository-wide validation both pass.
  - touches: none (validation and VM cleanup)
  - depends on: 2.2
