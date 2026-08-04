## Context

The configuration reference currently contains 153 keys. Existing Settings writes are atomic, single-key, and comment-preserving, while the existing scrollback editor launches a temporary file and deletes it on exit. The configuration editor must therefore reuse the pane/process pattern without reusing temporary-file cleanup or pretending that a client-local path is meaningful to a remote server.

## Goals / Non-Goals

**Goals:**

- Open the server-resolved config path in the user's editor.
- Preserve exact user bytes and last valid runtime state on invalid edits.
- Report unchanged, reloaded, invalid, and editor-failed outcomes with diagnostics.
- Work through the server/API boundary and present in the requesting TUI when available.

**Non-Goals:**

- A 153-field form, embedded TOML editor, or client-provided path.
- Automatic whole-file rewriting or formatting.
- Concurrent config editors or last-writer-wins Shepherd operations.

## Decisions

### Use a neutral server operation

Add `server.config.edit` with no path parameter. The server resolves `config_path()`, creates only a missing parent directory when necessary, and returns the created pane identity. A duplicate call returns the existing operation with `already_open = true`. If the server has no surface capable of hosting the pane, return a dedicated unavailable error.

Rejected: a TUI-private command or client-local path.

### Separate config editing from scrollback cleanup

Add dedicated platform editor argv helpers. Unix uses `VISUAL`, then `EDITOR`, then `vi`, passing the config path as a positional argument without interpolating it into an unquoted shell command. Windows reuses safe command-line parsing and falls back to Notepad. The config path is never placed in overlay `temp_files`.

Rejected: reusing the scrollback shell wrapper, which deletes its input file.

### Make exit status observable

Extend the internal pane-exit handoff so a config editor completion can distinguish a nonzero editor result from an unchanged file. The public completion event carries pane identity, outcome, and diagnostics; valid changed bytes may still reload even when the editor exits nonzero.

### Validate before applying

Snapshot existence and bytes before launch. On exit, compare final bytes. Changed valid TOML follows the existing reload pipeline. Invalid or unreadable changed content remains on disk while the last valid runtime configuration stays active. The result lets the user reopen the same file immediately.

## Risks / Trade-offs

- **Pane-exit plumbing touches shared runtime events** → characterize ordinary pane exits and config-editor exits separately.
- **Editor command parsing differs by platform** → retain platform-gated helpers and Linux/macOS/Windows argv tests.
- **External concurrent edits** → document that final bytes at editor exit are authoritative; Shepherd only prevents concurrent Shepherd-owned editors.
- **Missing config file** → create the parent directory but not a replacement file before launch.

## Migration Plan

1. Characterize current overlay exit restoration and config reload outcomes.
2. Add platform argv and server-owned operation state.
3. Add exit result/event handling and validated reload.
4. Add Settings action, generated API consumers, and next docs.
5. Rollback removes the action and optional API/event; user config bytes are never rewritten during rollback.

## Open Questions

None. The exact internal event shape may follow existing conventions, but path ownership, validation, and outcome semantics are fixed.
