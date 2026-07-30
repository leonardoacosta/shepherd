# Tasks — durable-session-persistence

Base commit: `1de05dc2`. Drift check: confirm `src/persist/io.rs:48-60` still
uses `target.with_extension("json.tmp")` + `std::fs::write` + `std::fs::rename`
with no `sync_all`, and that `src/detect/manifest_update.rs:401` still defines
`atomic_write`. If either has changed, STOP and report.

Exemplar to imitate: `src/detect/manifest_update.rs:401-425` (`atomic_write` —
unique tmp name, `sync_all`, rename, parent fsync).

## Ordered steps

1. **Extract a shared atomic-write helper.**
   - Move/duplicate `atomic_write` from `src/detect/manifest_update.rs` into a
     shared location (e.g. a new `src/persist/atomic.rs` or a small `fs` helper
     module). Keep the manifest-update caller working (re-point it or leave its
     copy and share the new one — do not change its behavior).
   - Gate: `just test-one manifest` still green (proves manifest path unbroken).

2. **Use it in `persist::io::save_json_to_path`.**
   - Replace the `with_extension("json.tmp")` + `write` + `rename` body with the
     shared helper. Preserve `resolve_write_target` and the `create_dir_all`.
   - Gate: `just test-one persist` and `just test-one restore` green.

3. **Enforce owner-only perms (unix).**
   - In the shared helper (or a wrapper), open the temp file with
     `OpenOptionsExt::mode(0o600)` under `#[cfg(unix)]`, and create the data dir
     `0700`. Exemplar for the dir-mode pattern: `src/remote/unix.rs` (search for
     an existing `0o700` / `set_permissions` on a herdr-owned dir) and
     `src/server/clipboard_image.rs` (owner-only file creation).
   - Do the same for `src/persist/plugin_registry.rs:39`.
   - Gate: new test asserts final mode `0o600` and dir `0o700` under
     `#[cfg(unix)]`.

4. **(Optional) order history before session in `save_to_paths`.**
   - At `src/persist/io.rs:63`, write `history_path` before `session_path` so a
     crash between renames can't leave a snapshot newer than its history.
   - Gate: existing `save_to_paths` tests green.

5. **Verify and close.**
   - `just check` passes. done-when: proposal `durable-session-persistence`
     archived.

## STOP conditions

- If lifting `atomic_write` would force a public API change on the
  detect module that other callers depend on, report before refactoring — a
  local copy in `persist` is an acceptable fallback.
- If any persist test depends on the exact `session.json.tmp` filename, report
  (the fixed name is the bug — the test encodes it).
