# Make session snapshot writes crash-durable and owner-only

Base commit: `1de05dc2` · Route: proposal · Effort: S · Confidence: HIGH · Category: correctness + security

## Why

Resolves advisory findings CORRECTNESS-02 and SECURITY-02 (audit against
`1de05dc2`).

Session persistence — the feature that restores the user's workspace/tab/pane
layout on restart — writes its snapshot without crash durability and with
default file permissions:

- `src/persist/io.rs:48-60` (`save_json_to_path`):
  ```rust
  let json = serde_json::to_string_pretty(snapshot)?;
  let tmp_path = target.with_extension("json.tmp");   // fixed, process-independent name
  std::fs::write(&tmp_path, &json)?;                    // no sync_all
  if let Err(err) = std::fs::rename(&tmp_path, &target) {
      let _ = std::fs::remove_file(&tmp_path);
      return Err(err);
  }
  ```
  No `sync_all` on the temp file, no parent-directory fsync, and a fixed
  `session.json.tmp` name (so two herdr processes sharing a data dir interleave
  into the same temp file). On power loss or SIGKILL after `rename` but before
  writeback, `session.json` can come back zero-length or torn.
- `src/persist/io.rs` `load()` then logs "failed to parse session file,
  ignoring" and the user silently loses their entire layout.
- `save_json_to_path` writes with default perms (`0644` under typical umask). The
  snapshot includes raw agent scrollback and pane launch argv — for AI coding
  agents this routinely contains tokens pasted into prompts and `.env` contents
  printed by commands. `docs/next/website/src/content/docs/session-state.mdx`
  already documents this and advises treating the directory "like terminal
  history" (conventionally `0600`), but the implementation does not enforce it.
- `src/persist/plugin_registry.rs:39` repeats the same weak write pattern.

**The correct pattern already exists in-repo** at
`src/detect/manifest_update.rs:401` (`atomic_write`): a pid+nanos-unique temp
name, `file.sync_all()`, `fs::rename`, then a parent-dir fsync. It simply was
never applied to the session files.

## What changes

1. Lift `atomic_write` (or an equivalent) into a shared fs helper and use it from
   `persist::io::save_json_to_path` and `persist::plugin_registry`.
2. Create the temp file `0600` and the containing data dir `0700` on unix.
3. Optionally write `session-history.json` before `session.json` so a crash
   between the two renames leaves history stale-but-older rather than a snapshot
   newer than its history (`src/persist/io.rs:63`, `save_to_paths`).

## Acceptance

- `just check` passes.
- A round-trip test writes a snapshot and asserts (a) the temp file is unique
  per process and (b) on unix the final file mode is `0600` and the data dir is
  `0700`.
- Existing persist/restore tests still pass (`just test-one persist`,
  `just test-one restore`).

## Out of scope

- The symlink-resolution behavior at `src/persist/io.rs:21`
  (`resolve_write_target`) must be preserved unchanged.
- Windows permission semantics differ; do not attempt unix mode bits under
  `#[cfg(windows)]` — gate the perms with `#[cfg(unix)]`.
