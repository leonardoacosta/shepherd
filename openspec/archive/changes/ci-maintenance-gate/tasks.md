# Tasks — ci-maintenance-gate

Base commit: `1de05dc2`. Drift check: confirm `.github/workflows/ci.yml` exists
and `justfile` `check` (`:41`) still lists the nine `scripts.test_*` unittest
modules. If the workflow structure has changed materially, STOP and report.

Exemplar to imitate: the existing job structure in `.github/workflows/ci.yml`
(steps: checkout, setup, run) and the exact unittest invocation in `justfile`
`check` recipe (`python3 -m unittest scripts.test_agent_detection_manifest_check
scripts.test_changelog ...`).

## Ordered steps

1. **Run the validators locally first to establish a clean baseline.**
   - `python3 -m unittest scripts.test_agent_detection_manifest_check
     scripts.test_changelog scripts.test_config_reference_check
     scripts.test_docs_translation_parity scripts.test_hermes_integration_asset
     scripts.test_package_windows_conpty scripts.test_preview
     scripts.test_vendor_libghostty_vt scripts.test_vendor_portable_pty`
   - `python3 scripts/config_reference_check.py`
   - `python3 scripts/agent_detection_manifest_check.py`
   - `python3 scripts/docs_translation_parity.py` (check its args — it may take
     the doc root; run for `docs/next/website` and the stable `website` root).
   - **STOP condition:** if any fails on the current head, that is a pre-existing
     drift — report it; decide with the maintainer whether to fix here or defer
     before gating.

2. **Add the `maintenance` job to `ci.yml`.**
   - ubuntu-latest, `actions/checkout`, `setup-python`, no Rust/Zig toolchain.
   - Run the same commands from step 1.
   - Trigger on the same `pull_request` (and default-branch push) events the
     existing jobs use.

3. **Verify the job passes in CI.**
   - Gate: the job is green on a PR from this branch.
   - done-when: proposal `ci-maintenance-gate` archived.

## Notes

- Keep it Python-only so it stays ~1 minute; do not add `cargo`/`zig` steps.
- Do not make it a required status check until it has been green for at least one
  cycle (record that decision for the maintainer).
