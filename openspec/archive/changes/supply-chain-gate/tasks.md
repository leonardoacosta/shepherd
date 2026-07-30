# Tasks — supply-chain-gate

Base commit: `1de05dc2`. Drift check: confirm `Cargo.toml` still pins
`portable-pty = "=0.9.0"` with `[patch.crates-io]`, that
`scripts/test_vendor_portable_pty.py` / `scripts/test_vendor_libghostty_vt.py`
exist, and that `Cargo.lock` still contains `termwiz`, `winapi`, `winreg`. If the
dependency graph has materially changed, re-derive the duplicate list before
acting.

Exemplar: `scripts/test_vendor_libghostty_vt.py` and
`scripts/test_vendor_portable_pty.py` (vendored-base verification style) for the
drift check; `.github/workflows/ci.yml` job structure for the CVE gate.

## Ordered steps (land 1-2 first; 3 is separate, verified increments)

1. **Add the CVE gate.** Add a CI step running `cargo audit` (install
   `cargo-audit`, or use `cargo deny check advisories`). Configure to fail on
   critical/high affecting runtime/build paths; ignore-list low-signal advisories
   with recorded reasons.
   - Gate: the job is green on current head (triage any real hit before gating).

2. **Add the fork-drift check.** Extend the vendor test scripts (or a scheduled
   workflow) to compare the recorded base (`vendor/libghostty-vt.vendor.json`
   `source_commit`; `portable-pty` base in `.patches.md`) against latest upstream
   and warn/open an issue on divergence.
   - Gate: `python3 -m unittest scripts.test_vendor_libghostty_vt
     scripts.test_vendor_portable_pty` passes.

3. **De-dup (each a separate commit, verified).**
   - `cargo tree -d` and `cargo tree -i <crate>` for `termwiz`, `winapi`,
     `winreg`, `memmem`, `fancy-regex` — record what pulls each.
   - Drop the `ratatui` termwiz backend feature if `cargo tree -i termwiz` shows
     it's only reachable through an unused feature.
   - Unify `nix` on one major where safe.
   - Fold `winapi`/`winreg` removal into a `portable-pty` fork refresh (its own
     change, per AGENTS.md vendoring rules).
   - Gate per increment: `just check` (unix + windows-target clippy) passes.

4. **Verify and close.**
   - `just check` passes. done-when: proposal `supply-chain-gate` archived (de-dup
     increments may be tracked as follow-ups if deferred).
