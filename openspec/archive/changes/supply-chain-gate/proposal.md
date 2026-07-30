# Add a supply-chain gate: cargo audit, vendored-fork drift, dependency de-dup

Base commit: `1de05dc2` · Route: proposal · Effort: M (L if the de-dup work is pursued) · Confidence: MED — lockfile facts are certain; "droppable" needs `cargo tree` verification · Category: dependencies / security

## Why

Adjacent follow-on resolving advisory findings DEPS-01, DEPS-02, DEPS-03, and a
rider on `ci-maintenance-gate` (it creates the CI slot). No advisory-database
check runs today (the audit could not run `cargo audit` in its sandbox), and two
critical-path forks drift silently.

- **No CVE gate.** Nothing runs `cargo audit`. `Cargo.lock` has 244 packages,
  none advisory-checked.
- **Vendored-fork drift.** `Cargo.toml:31,46-47` — `portable-pty = "=0.9.0"` with
  `[patch.crates-io]` → `vendor/portable-pty`; `vendor/portable-pty.patches.md`
  documents two active patches. `vendor/libghostty-vt.vendor.json` pins a
  `source_commit`. `scripts/test_vendor_portable_pty.py` /
  `scripts/test_vendor_libghostty_vt.py` verify patches apply, but nothing
  *notices* when upstream publishes a fix — both forks (PTY spawn + the VT parser
  that consumes untrusted agent output) drift silently.
- **Duplicate stacks** in `Cargo.lock`: a whole second terminal-emulation stack
  (`termwiz 0.23.3`, `terminfo`, `vtparse`, `wezterm-*`) pulled via
  `ratatui-termwiz`/`portable-pty` while herdr vendors libghostty-vt as its VT
  impl; stale Windows-path crates `winapi 0.3.9` + `winreg 0.10.1` alongside the
  modern `windows-sys 0.61.2`; abandoned `memmem 0.1.1`; `fancy-regex` next to
  `regex`; and duplicate majors (`nix` 0.28/0.29/0.31, `bitflags` 1/2, `syn` 1/2,
  `thiserror` 1/2, `getrandom` 0.3/0.4, `hashbrown` 0.15/0.16). This is audit
  surface and future-CVE surface that buys nothing.

## What changes

1. **CVE gate.** Add `cargo audit` (or `cargo deny`) to CI — ideally the
   `maintenance` job from `ci-maintenance-gate` if a Rust-toolchain-free variant
   isn't feasible, else a dedicated small job. Report only critical/high advisories
   affecting reachable runtime/build paths (avoid low-signal noise).
2. **Fork-drift check.** Extend `scripts/test_vendor_*.py` (or add a scheduled
   job) to compare the recorded vendored base against the latest upstream
   release/commit and flag divergence.
3. **De-dup (staged, verify first).** Run `cargo tree -d` / `cargo tree -i` to
   confirm what pulls `termwiz`, `winapi`, `winreg`, `memmem`, `fancy-regex`; drop
   the `ratatui` termwiz backend feature if unused; unify `nix` on one major; fold
   the `winapi`/`winreg` removal into the next `portable-pty` fork refresh.

## Acceptance

- CI runs an advisory check and is green (or its findings are triaged and either
  fixed or explicitly waived with a recorded reason).
- A drift check exists that flags when `portable-pty` / libghostty-vt upstream has
  advanced past the vendored base.
- If de-dup is pursued: `cargo tree -d` shows the targeted duplicates removed and
  `just check` (incl. windows-target clippy) still passes.

## Out of scope / staging

- The de-dup touches the PTY backend and ratatui backend selection — the highest-
  blast-radius area in the tree. Stage it: land the gate + drift check first
  (steps 1-2, low risk), then pursue de-dup as separate verified increments. Do
  **not** bundle a `portable-pty` fork bump with the CVE gate.

## STOP conditions

- If `cargo audit` surfaces an unfixable advisory in a vendored fork, STOP and
  report — the response is a fork bump or a documented waiver, a maintainer call,
  not something to improvise.
- If `cargo tree -i` shows a targeted "duplicate" is actually required by a
  distinct feature path, leave it and record why.
