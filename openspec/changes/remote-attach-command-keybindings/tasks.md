# Tasks — restore server-owned command keybindings under a local-keybinding attach

Base commit: **`636f967c`** (branch `dev`). Confirm the excerpts in `proposal.md` still match
their cited `file:line` sites before starting; if the code has drifted, STOP and report it.

Gate commands are `cargo nextest run --bin shepherd <filter>` (the crate has no lib target, so a
bare `cargo nextest run <filter>` resolves nothing). `just check` before committing, per
`AGENTS.md`. No `unwrap()` in production code.

## 1. Carry the server's command entries to the resolution point

- [ ] 1.1 In `src/server/headless.rs`, capture the server's raw command entries from the loaded
      configuration — `keys.command`, typed `Vec<CommandKeybindConfig>` — next to where
      `server_keybindings` is built at `src/server/headless.rs:433`. Wrap them in an `Arc` so each
      client handshake thread can hold a cheap clone. The raw entries are required, not the
      resolved `Keybinds::custom_commands`, because task 2 re-resolves them through the binding
      registry alongside the client's actions.
- [ ] 1.2 Thread that `Arc` into the handshake. The accept loop at `src/server/headless.rs:387`
      spawns `handle_client_handshake`; capture the clone in that closure. Note the accept loop is
      spawned before line 433 in source order — hoist the capture above the spawn rather than
      reordering the loop.
- [ ] 1.3 Add the same parameter to the Windows accept path at `src/server/client_accept.rs:101`.
      Both call sites must pass the server's entries; neither may pass an empty vector as a
      placeholder.
- [ ] 1.4 Extend `handle_client_handshake` (`src/server/client_transport.rs:514`) to accept the
      new parameter and forward it to `parse_client_keybindings`. Update the two in-file test call
      sites at `src/server/client_transport.rs:1265` and `:1340`.
- [ ] 1.5 Gate: `cargo nextest run --bin shepherd handshake` passes with no signature-related
      failures.

## 2. Resolve client actions and server commands through one registry

- [ ] 2.1 In `parse_client_keybindings` (`src/server/client_transport.rs:440`), replace
      `config.keys.command.clear()` at line 447 with an assignment of the server's entries onto
      the deserialized client config's `keys.command`. Assignment, not extension — the client's
      own entries must be dropped, and an assignment makes that structurally impossible to skip.
- [ ] 2.2 Leave the subsequent `config.prefix_key()` / `config.keybinds()` calls untouched. The
      chord precedence required by the spec is whatever the existing binding registry already does
      when a `[[keys.command]]` entry and a built-in action claim one chord — see
      `RegisteredBinding::conflict` (`src/config/keybinds.rs:410`) and the shipped test
      `user_binding_silently_displaces_default_binding` (`src/config/keybinds.rs:2144`). Do not
      write a second precedence path.
- [ ] 2.3 Update `parse_client_keybindings_accepts_local_profile`
      (`src/server/client_transport.rs:1211`). Its current
      `assert!(keybindings.keybinds.custom_commands.is_empty())` asserts the defect. Replace it
      with: passing no server entries yields no custom commands; passing a server entry yields
      exactly that command; the client payload's own `lazygit` entry never appears.
- [ ] 2.4 Add a test that a server command chord colliding with a client action chord resolves to
      the command, and that a non-colliding client action survives untouched.
- [ ] 2.5 Gate: `cargo nextest run --bin shepherd parse_client_keybindings` passes, including the
      two new assertions.

## 3. Confirm the applied set at the foreground-client seam

- [ ] 3.1 Read `sync_foreground_client_state` (`src/server/headless.rs:1034`). With task 2 done,
      `client.keybindings` already carries the merged set and `apply_keybindings` at line 1045
      needs no change. Confirm this by inspection and record the finding in the task list — do not
      add a second merge here.
- [ ] 3.2 Confirm the detach path still restores the pure server set: `apply_keybindings` with
      `self.server_keybindings` at `src/server/headless.rs:993`, `:1004`, and `:1150` must remain
      unmodified, so a disconnect returns the session to server-owned actions *and* commands.
- [ ] 3.3 Gate: `cargo nextest run --bin shepherd foreground_client` passes.

## 4. Report what was not sent

- [ ] 4.1 In `src/client/mod.rs`, at `requested_keybindings` (`src/client/mod.rs:699`), detect
      whether the loaded local config declares any `[[keys.command]]` entries before building the
      profile. The profile itself omits them, so the count must be read from the config, not
      recovered from the serialized TOML.
- [ ] 4.2 When the count is non-zero and the mode is local, emit one line to stderr before attach
      naming the count and stating that the server's own command bindings apply instead. Emit
      nothing when the count is zero or the mode is server.
- [ ] 4.3 Check whether `sync_visible_server_config_diagnostic`
      (`src/server/headless.rs:1046`) is the better surface for this notice given it already
      takes `uses_local_keybindings`. If it is, implement there instead and record why; a
      transient stderr line before a full-screen attach may be scrolled away immediately.
- [ ] 4.4 Gate: attach a client whose config declares command bindings using local keybindings and
      paste the notice. Attach one with none and paste the empty result.

## 5. State the rule in help and docs

- [ ] 5.1 Extend the `--remote-keybindings` help text (`src/main.rs:681`) to name what each mode
      takes from which side, commands included.
- [ ] 5.2 Rewrite `docs/next/website/src/content/docs/persistence-remote.mdx:49`. The current
      sentence stops at "Local custom command keybindings are not sent, because those commands
      would run on the remote host" — which is true and reads as though the server's own bindings
      were unaffected. State that the server's command bindings apply, and that a server command
      wins a chord it shares with a client action binding.
- [ ] 5.3 Apply the same correction to every maintained locale carrying that page — check
      `docs/next/website/src/content/docs/ja/` and `docs/next/website/src/content/docs/zh-cn/`.
- [ ] 5.4 Add a `### Fixed` entry to `docs/next/CHANGELOG.md`. No protocol bump: say so, since the
      neighbouring `--remote-keybindings` entry at line 445 announced protocol 9 and a reader will
      look for one.
- [ ] 5.5 Gate: `rg -n "not sent" docs/next/website/src/content/docs` shows every locale's line
      carrying both halves of the rule.

## 6. Close out

- [ ] 6.1 Gate: `just check` passes; paste the summary line.
- [ ] 6.2 Propose the commit message and wait for approval before committing, per `AGENTS.md`.
