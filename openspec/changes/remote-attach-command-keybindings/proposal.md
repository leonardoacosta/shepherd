# Restore server-owned command keybindings under a local-keybinding remote attach

Base commit: **`636f967c`** (branch `dev`).

## Why

`shepherd --remote <target>` defaults to `--remote-keybindings local`. In that mode the client
serializes a keybindings profile and the server adopts it **in place of** its own live keybind
set. Two independent code paths strip `[[keys.command]]` from that path:

- `Config::local_profile` (`src/config/model.rs:625`) copies `prefix` and every built-in action
  field, then ends at `copy_user_field!(indexed)`. It never copies the `command` vector, so
  custom command bindings do not leave the client.
- `parse_client_keybindings` (`src/server/client_transport.rs:447`) calls
  `config.keys.command.clear()` on the received TOML before building the `LiveKeybindConfig`.

Both are correct on their own terms, and the second is a genuine security boundary: an attaching
client must not be able to inject arbitrary shell execution onto the server host. The docs already
state the client half — `docs/next/website/src/content/docs/persistence-remote.mdx:49`: "Local
custom command keybindings are not sent, because those commands would run on the remote host."

The defect is the *unstated* consequence. Because the client profile **replaces** the live
keybind set rather than merging into it, the server's own `[[keys.command]]` entries — read from
the server's own `config.toml`, owned by the person who administers that host, already trusted to
run there — are suppressed along with the client's. The shipped test at
`src/server/client_transport.rs:1234` asserts the end state directly:

```rust
assert!(keybindings.keybinds.custom_commands.is_empty());
```

So a user whose server config binds plugin actions to `prefix+f`, `prefix+t`, `prefix+c`,
`prefix+w` gets none of them on a default remote attach, and no documentation anywhere predicts
that. Three properties make it hard to diagnose:

1. **It fails silently.** The config parses clean on both hosts; no diagnostic reaches the client,
   the server log, or the help panel.
2. **It fails misleadingly.** A command binding that displaces a built-in on the same chord does
   not go dead — it reverts. `prefix+c` becomes `new_tab` and `prefix+w` becomes
   `workspace_picker`, so the key does *something*, and the user reads it as a broken plugin
   rather than a suppressed binding.
3. **The workaround inverts a second choice.** `--remote-keybindings server` restores the
   commands, but only by discarding the local keybinding snapshot the default exists to provide.
   There is currently no way to hold both, and nothing tells the user they are trading.

The security boundary does not require this outcome. Refusing *client-supplied* commands and
honoring *server-owned* commands are separable, and the server already holds its own parsed
config when it applies the client profile.

## What Changes

- Under a local-keybinding attach, populate `Keybinds::custom_commands` from the **server's own**
  configuration instead of leaving the vector empty. The client's action bindings and prefix
  continue to come from the client profile.
- Keep `config.keys.command.clear()` on the received TOML. Client-supplied commands stay refused;
  this change adds no path for a client to name a command the server will run.
- Define chord precedence when a server command and a client action bind the same chord: the
  server command wins. This reproduces what the same config does in a local session on that host,
  where a `[[keys.command]]` entry displaces a built-in action bound to the same chord.
- Report the boundary instead of leaving it silent. When a local-keybinding attach discards
  command bindings present in the *client's* config, the client emits a one-line notice naming
  the count and the reason.
- State the full rule in `--help` (`src/main.rs:681`) and in `persistence-remote.mdx`, replacing
  the half-statement at line 49: local commands are not sent, server commands still apply, and
  server commands win a chord collision.

No protocol change. `ClientKeybindings` and the `Hello` message are untouched, the merge is
server-side, and a client at the current protocol version gets the corrected behavior without a
version bump.

## Rejected alternative

**Provenance-aware precedence** — let a client binding the user explicitly configured beat a
server command on the same chord, and only let the server command displace a client *default*.
This is the more precise rule, and `local_profile` already computes that distinction locally via
`user_fields` and `binding_config_is_effective`. It is rejected here because the distinction does
not survive the wire: `local_profile` writes user-set fields and effective defaults into the same
flat `[keys]` table, so after the server deserializes it, every field looks user-set. Carrying
provenance would need a new protocol field and a version bump — a larger change than the defect
warrants, and one that can land later on top of this without rework.

## Acceptance

- A remote attach with default keybindings fires the server's `[[keys.command]]` bindings.
- A client-supplied `[[keys.command]]` entry is never executed by the server.
- A chord bound by both a server command and a client action runs the server command.
- A client whose config carries command bindings sees a notice on attach saying they were not
  sent.
- `--help` and `persistence-remote.mdx` state which side owns commands and who wins a collision.
- `just check` passes.
