# Tasks — fix-contributing-docs

Base commit: `1de05dc2`. Drift check: confirm `CONTRIBUTING.md:86` still claims
the pre-commit hook runs only `cargo fmt --check`, and `justfile:16-18,27-30`
still define `lint` and `ci` as described. If the doc was already corrected, STOP.

Exemplar: the actual recipe bodies in `justfile` (`lint`, `ci`, `test`, `check`)
are the source of truth the docs must match; `flake.nix:88-99` devShell
`buildInputs`/`packages` list is the shape to extend.

## Ordered steps

1. **Correct the two command descriptions** in `CONTRIBUTING.md` to match
   `justfile` (pre-commit → `just lint` = fmt + clippy; `just ci` → fmt + nextest
   + bun asset tests + plugin-marketplace bun test).

2. **Add a prerequisites section** to `CONTRIBUTING.md`: zig 0.15.2, bun, python3,
   `just`, `cargo-nextest`, rust (cite `rust-toolchain.toml`), with `nix develop`
   and `mise` as alternative setup paths. Cross-link from `README.md` build
   section.

3. **Add bun + python3 to the flake devShell** (`flake.nix`) so `just ci`/`just
   test` work inside `nix develop`.
   - Gate: on a nix machine, `nix develop -c just test` runs (or record the
     devShell now lists bun+python3 and hand live verification to the maintainer).

4. **(Optional) friendlier build.rs error.** Replace the bare zig-spawn panic in
   `build.rs:65-80` with a message pointing at the prerequisites section.
   - Gate: `just lint` clean if touched.

5. **Verify and close.**
   - `just check` passes. done-when: proposal `fix-contributing-docs` archived.

## STOP conditions

- If the flake is intentionally minimal (e.g. bun is expected via a separate
  path), confirm with the maintainer before adding it rather than assuming.
