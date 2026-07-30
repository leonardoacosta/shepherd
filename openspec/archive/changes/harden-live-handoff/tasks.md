# Tasks — harden-live-handoff

Base commit: `1de05dc2`. Drift check: confirm `src/server/headless.rs:1052` still
builds the token via `format!("{}-{}", std::process::id(), ...nanos)`, that
`src/server/handoff.rs:74-80` passes `token` as `.arg(token)`, and that
`ServerLiveHandoffParams.import_exe` is spawned verbatim. If changed, STOP and
report.

Exemplar: `src/update.rs:~1475` (the sole legitimate caller passing the verified
update binary) — validation must not break this path. For CSPRNG usage, search
the tree for existing `getrandom` call sites and match their style.

## Ordered steps

1. **Replace the token generator.**
   - Generate ≥16 random bytes from the OS CSPRNG, hex/base64-encode. Confirm
     `getrandom` (or another CSPRNG already present) is in `Cargo.lock` before
     using; do not add a dependency if one exists.
   - Gate: `just test-one handoff` green.

2. **Pass the token off-argv.**
   - Change `spawn_handoff_import` to pass the token via env var (e.g.
     `HERDR_HANDOFF_TOKEN`) or an inherited pipe fd instead of `.arg(token)`.
   - Update the import child read site (`src/server/headless.rs`, currently
     reading `args[4]`) to read from the same channel.
   - Gate: `tests/live_handoff.rs` green (round-trip still works).

3. **Constant-time comparison.**
   - Replace the plain `!=` token check (`src/server/handoff.rs:~150`) with a
     constant-time equality. A small hand-rolled byte-wise compare is fine if no
     crate is present.
   - Gate: `just test-one handoff` green.

4. **Validate `import_exe`.**
   - Before spawn (`src/server/headless.rs:~1050` / `spawn_handoff_import`),
     reject a non-absolute path, a non-regular file, or a symlink, and require it
     to resolve within `std::env::current_exe()`'s directory (or the
     update-verified path). Return a clear API error otherwise.
   - Add a test: an `import_exe` outside the allowed dir is rejected.
   - Add a test: the spawned child's argv does not contain the token.
   - Gate: `just test-one handoff` + `just test-one live_handoff` green.

5. **Verify and close.**
   - `just check` passes. done-when: proposal `harden-live-handoff` archived.

## STOP conditions

- If the update flow passes `import_exe` as something *other* than a path inside
  `current_exe()`'s dir (e.g. a temp download path), capture what it actually
  passes and widen the allow-rule to exactly that shape — do not loosen to "any
  absolute path".
