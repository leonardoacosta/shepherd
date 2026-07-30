# Spike: socket-level auth / capability tiering  — DEFERRED (backlog)

Base commit: `1de05dc2` · Route: bead-equivalent spike · Status: **DEFERRED / not scheduled** · Category: security

> Recorded as a deferred spike ("bead") for later, per maintainer request. herdr
> tracks work in `openspec/`, not a beads DB, so this stub lives here as the
> bead-equivalent: a short, tracked, unscheduled placeholder — **not** a full
> proposal. Promote it to a real proposal (add a `tasks.md`, drop the DEFERRED
> banner) when it's picked up. No `tasks.md` intentionally.

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

- `src/api/server.rs:~139-200` — connections served with no authentication, no
  capability negotiation, no peer check.
- `src/server/client_accept.rs:19-45` — client handshake likewise unauthenticated.
- `src/ipc.rs:~277` — access control is post-hoc file-permission restriction only;
  `restrict_socket_permissions` is a no-op on Windows (`:283`).
- Powerful methods on the same flat surface: `server.live_handoff`
  (`src/api/schema.rs:50`), plugin link/enable/invoke (`:216-238`).

## Why deferred, not now

- It overlaps with and should absorb the narrower fixes already routed
  (`harden-live-handoff` `import_exe` validation; `spike-plugin-install-api`
  consent gate) — better designed once those land and reveal the real method-tier
  boundaries.
- A capability model touches every method dispatch site; premature design risks
  churn. Land the point-fixes first, then generalize.

## When to promote

Pick this up after `harden-live-handoff` and `spike-plugin-install-api` land.
Promotion = write `tasks.md` (peer-cred check first as an independently shippable,
low-risk increment; then the method-tier design), remove the DEFERRED banner.
