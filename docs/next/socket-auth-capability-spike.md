# Socket Auth / Capability Tiering Spike

Date: 2026-07-30
Status: peer-uid check shipped; tiering is design only, pending enforcement
Scope: design spike for socket-level access control (openspec `spike-socket-auth-capability`)

## Current state

Until this spike, file permissions were the entire access-control story on the
Shepherd socket API: `0600` mode on unix, the default DACL on Windows
(`src/ipc.rs::restrict_socket_permissions`, unix arm `src/ipc.rs:352-356`,
Windows no-op `src/ipc.rs:358-361`). There was no peer-credential check on
accept and no per-method authorization anywhere in `src/api/` or
`src/server/client_accept.rs` — any process running as the same user could
open the socket and call any method. As the surface gains more powerful
methods (a possible `pane.output` stream, `plugin.install`, the exec-adjacent
`server.live_handoff`), "any same-uid process can do anything" is a weaker
story, and shepherd's whole point is hosting agents that are same-uid.

## Platform feasibility finding

- **Linux**: `SO_PEERCRED` via `getsockopt` on the raw fd is implementable
  today. `libc = "0.2"` (`Cargo.toml:30`) already exposes `libc::ucred` /
  `SO_PEERCRED`, and `src/ipc.rs` already extracts the raw fd from
  `LocalStream::UdSocket` via `AsRawFd` for the existing `probe_stream_closed`
  unix arm (`src/ipc.rs:158-176`) — no new dependency.
- **macOS**: `getpeereid` is the equivalent primitive, same shape: raw fd from
  `AsRawFd`, one `libc` call, no new dependency.
- **Windows**: not a hard dead end, but meaningfully more code and one
  missing feature flag. The real mechanism is `GetNamedPipeClientProcessId`
  to get the connecting client's process id, then `OpenProcessToken` +
  `GetTokenInformation(TokenUser)` to compare that process's SID against the
  server's own. `Cargo.toml`'s `[target.'cfg(windows)'.dependencies]` block
  (`Cargo.toml:49-67`) enables 16 `windows-sys` features and none of them is
  `Win32_Security` or a `Win32_Security_*` sub-feature — that flag is not
  currently enabled. Enabling it is a one-line change, but the SID-comparison
  call sequence is several times more code than the two Unix syscalls, and
  even with it, the check cannot cleanly distinguish "same interactive
  session, different (possibly untrusted) process" from "genuinely different
  user" without session-enumeration APIs (e.g. `WTSEnumerateSessions`)
  outside the current dependency tree. A check that only looks real is worse
  than an explicit documented gap, so Windows peer verification was scoped
  out rather than shipped in a weaker form.

## What shipped in this spike

This was not just a design doc — the peer-uid check landed as code, on both
accept paths:

- `src/api/server.rs::handle_incoming_connection` (`src/api/server.rs:144`)
  authorizes the peer first, before `try_acquire_connection_slot` and before
  `handle_connection` dispatch — an unauthorized peer never consumes a
  connection slot or reaches request dispatch.
- `src/server/client_accept.rs::accept_pending_client_connections_with_limit`
  (`src/server/client_accept.rs:49`) applies the same ordering to the
  thin-client handshake path.

Both call `crate::ipc::peer_uid_authorized` (`src/ipc.rs`), which has a unix
arm (`SO_PEERCRED`-based, comparing the peer uid against
`libc::getuid()`) and a Windows arm that always returns `Ok(true)` — an
explicit, documented no-op rather than a silent skip, matching
`restrict_socket_permissions`'s existing Windows no-op precedent and the
unix-only handoff-token gate landed in `c000681f` ("fix: gate handoff import
token on unix"). The check **fails closed**: if the authorization call
itself errors (`Err(err)`), the connection is rejected the same as an
explicit uid mismatch, not treated as an ambiguous "allow" case.

## Method capability tiering (summary)

The spike also tiers all 91 client-facing `Method` variants
(`src/api/schema.rs:45-238`) into read-only / mutate / exec, so a future
enforcement layer has a model to build on: **28 read-only, 54 mutate, 9
exec**. Three additional variants (`PaneGraphicsStreamSet`,
`PaneGraphicsStreamOpen`, `PaneGraphicsStreamClose`) are internal
re-dispatch targets that never appear on the wire and are excluded from the
count. The full 91-row table lives in
`openspec/changes/spike-socket-auth-capability/design.md` § Section B — this
doc does not reproduce it.

Two tiering calls are worth calling out because they are not obvious from
the method name alone:

- **`agent.start`** is tiered exec because `AgentStartParams.args: Vec<String>`
  is attacker-controllable process argv — starting an agent is equivalent to
  spawning an arbitrary process with arbitrary arguments.
- **`layout.apply`** is tiered exec, and is the highest-blast-radius method in
  the inventory: `LayoutApplyParams.root: LayoutNode` recurses into
  `LayoutPane.command: Option<Vec<String>>`, so one call can launch an
  arbitrary command with arbitrary args in each pane across a whole layout
  tree — the same risk class as `agent.start`, but for a whole tree of panes
  instead of one.

## Open follow-ups

- **No dispatch-site enforcement exists yet.** The tiering table is a design
  artifact only; no code today consults it to gate a method call.
- **`PlatformCapabilities::peer_credential_check`** (`src/platform/mod.rs:54`,
  set via `cfg!(unix)` at `src/platform/mod.rs:63`) is declared but not
  consulted by the shipped check — `peer_uid_authorized` is gated directly by
  `#[cfg(unix)]` / `#[cfg(windows)]`, not by reading this flag. The field is
  currently unread by any caller.
- **Tiering mismatch with `spike-plugin-install-api`, flagged not absorbed.**
  This spike tiers `plugin.link` / `plugin.enable` as ordinary `mutate` — the
  same tier as `tab.rename`. The sibling spike's threat model treats those two
  methods as materially higher risk, because they gate *future* arbitrary code
  execution (a plugin's `[[startup]]` hooks) rather than making an immediate
  bounded state change. The recommended resolution, left for whichever spike
  lands the confirmation mechanism: model it as a `requires_confirmation` tier
  *modifier* on `plugin.link` / `plugin.enable`, not a fourth tier and not a
  silently-absorbed exception.

Full detail, evidence citations, and the reconciliation with `c000681f` and
`spike-plugin-install-api`:
`openspec/changes/spike-socket-auth-capability/design.md`.
