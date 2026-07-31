# Spike: socket-level auth / capability tiering

Base commit: `c000681f` · Route: design spike · Category: security

## The idea (one paragraph)

Today file permissions are the entire access-control story on the socket API:
`0600` mode (unix) / default DACL (windows), with no peer-credential check and no
per-method authorization anywhere in `src/api/` or `src/server/client_accept.rs`.
As the surface gains more powerful methods (the `pane.output` stream, a possible
`plugin.install`, the exec-adjacent `server.live_handoff`), "any same-uid process
can do anything" becomes a weaker story — and herdr's whole point is hosting
agents that are same-uid. This spike would design (a) a `SO_PEERCRED` /
`getpeereid` peer check on accept (reject peers whose uid differs from the
server's), and (b) a capability tiering of methods — read-only vs mutate vs exec —
so the hardening done piecemeal in `harden-live-handoff` and
`spike-plugin-install-api` becomes a coherent model instead of per-method patches.

## Why it's grounded (evidence)

- `src/api/server.rs:85-152` (`start_server_with_capabilities`, accept loop
  :101-144) — connections served with no authentication, no capability
  negotiation, no peer check; `:195-237` (`handle_connection`) dispatches every
  method with no per-call authorization. A connection-count limit landed since
  the original evidence pass (`MAX_API_CONNECTIONS`, :34) but it bounds
  concurrency, not identity — it does not change this finding.
- `src/server/client_accept.rs:45-95`
  (`accept_pending_client_connections_with_limit`) — client handshake likewise
  unauthenticated; the handshake spawn itself is at `:74-84`. Same drift as
  above: a connection limit was added, no peer check.
- `src/ipc.rs:287-292` (`restrict_socket_permissions`, unix) — access control is
  post-hoc file-permission restriction only; `:294-297` is the Windows no-op
  (unchanged in shape, just moved a few lines from prior citation).
- Powerful methods on the same flat surface: `server.live_handoff`
  (`src/api/schema.rs:50`, citation unchanged), plugin link/enable/invoke
  (`:216-237`). `c000681f` ("fix: gate handoff import token on unix") narrowed
  `live_handoff`'s import path to `#[cfg(unix)]` only
  (`src/server/headless.rs::handoff_import_token`) rather than adding any
  peer-identity check — the shared-secret token gate this spike would sit
  alongside is now explicitly unix-only, which the peer-cred design (also
  unix-first, see `tasks.md`) should track rather than diverge from.

## Sequencing

This spike is promoted ahead of its original "land the point-fixes first"
sequencing, per explicit maintainer request. It should still absorb, not
duplicate, the narrower fixes running alongside it: `harden-live-handoff`
already landed the unix-only token gate on the `live_handoff` import path
(`c000681f`) — the capability-tier design in `tasks.md` treats that as a fixed
constraint, not something to redesign. `spike-plugin-install-api` is running in
this same batch and is deciding the `plugin.link`/`plugin.enable` consent gate;
this spike's method tiering should classify those methods consistent with
whatever that spike lands, cross-checked explicitly in `tasks.md` step 4 rather
than assumed.
