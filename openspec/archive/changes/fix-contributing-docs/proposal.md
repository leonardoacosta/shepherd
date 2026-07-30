# Fix CONTRIBUTING.md verification commands and document prerequisites

Base commit: `1de05dc2` · Route: proposal · Effort: S · Confidence: HIGH · Category: DX / docs

## Why

Resolves advisory findings DX-01 and DX-02 (audit against `1de05dc2`). For a
project whose stated goal is staying manageable for a solo maintainer, every one
of these is a support conversation.

Both documented verification commands are wrong, and no prerequisites are listed:

- `CONTRIBUTING.md:86` says "The pre-commit hook runs `cargo fmt --check` before
  every commit." `.githooks/pre-commit:7` actually runs `just lint` =
  `cargo fmt --check` **plus** `cargo clippy --all-targets --locked -- -D warnings`
  (`justfile:16-18`).
- `CONTRIBUTING.md:94` says "`just ci` runs `cargo fmt --check` and
  `cargo nextest run`." `justfile:27-30` shows `just ci` also runs
  `integration-assets-test` (two `bun test` invocations) and
  `plugin-marketplace-test` (`cd workers/plugin-marketplace && bun test`).
- Nowhere in `CONTRIBUTING.md` or `README.md:70-79` are zig, bun, python3, `just`,
  or `cargo-nextest` named as prerequisites — yet `build.rs:65-80` unconditionally
  shells out to `zig build` and panics with `failed to execute zig build for
  vendored libghostty-vt` if zig is absent. A newcomer's first command
  (`cargo build --release`) panics on missing zig with no install hint.

Toolchain pinning is also inconsistent (DX-02): `rust-toolchain.toml` pins rust
1.96.1; `mise.toml` pins only zig; `flake.nix:88-99` provides a devShell without
bun or python3, so `just ci`/`just check` fail inside `nix develop`.

## What changes

1. Add a prerequisites block to `CONTRIBUTING.md` naming zig 0.15.2, bun, python3,
   `just`, `cargo-nextest`, and rust (from `rust-toolchain.toml`), with the
   `nix develop` and `mise` paths as alternatives.
2. Correct the two command descriptions to match `justfile`.
3. Add `bun` and `python3` to the flake devShell so the repo's own verification
   commands run inside `nix develop`.
4. Optionally: make `build.rs` emit "install zig 0.15.2 (see CONTRIBUTING.md)" on
   the spawn failure instead of a bare panic.

## Acceptance

- `CONTRIBUTING.md` command descriptions match `justfile` exactly (a reader
  running the documented commands sees what the doc says they'll see).
- A prerequisites section lists every tool `just ci` / `cargo build` requires.
- `just ci` (or at least `just test`) runs to completion inside `nix develop`
  after the devShell change — verify on a machine with nix, or record that the
  devShell now includes bun+python3 and defer live verification to the maintainer.
- `just check` still passes (this is mostly docs + nix; no Rust logic changes
  unless the optional `build.rs` message is added).

## Out of scope

- Reworking the pre-commit hook to be faster (DX-03) — that is a separate change;
  here only *document* what the hook actually does.
- Consolidating the triplicated `RUST_TOOLCHAIN_VERSION` across workflows — note
  it as follow-up, don't do it here.
