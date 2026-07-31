# Tasks — spike-socket-auth-capability (design spike)

Base commit: `c000681f`. Drift check: confirm `src/api/server.rs`'s accept loop
(`start_server_with_capabilities`, now spanning lines 85-152 after the
connection-limit change) still has no peer-credential check before
`handle_connection` dispatch; confirm `src/server/client_accept.rs`'s handshake
spawn (now inside `accept_pending_client_connections_with_limit`, lines 45-95)
is likewise still unchecked; confirm `src/ipc.rs::restrict_socket_permissions`
is still a hard no-op on Windows (`:294-297`) with no DACL restriction attempted;
confirm `src/server/handoff.rs` still gates its shared-secret token exchange to
`#[cfg(unix)]` only (`src/server/headless.rs::handoff_import_token`, landed in
`c000681f`) rather than adding any cross-platform peer check. If any of these
have changed, re-scope before continuing.

Exemplar: `src/ipc.rs::probe_stream_closed` (two `#[cfg(unix)]` /
`#[cfg(windows)]` implementations, unix using `AsRawFd` + a raw `libc::recv`
call, windows using `windows_sys::Win32::System::Pipes`) is the in-repo pattern
for a platform-split raw-socket primitive on `LocalStream` — imitate its shape
for the peer-credential check. `platform::capabilities()` /
`PlatformCapabilities` in `src/platform/mod.rs:44-51` is the existing
capability-flag pattern (`live_handoff: cfg!(unix)`) to extend rather than
reinvent when the Windows answer turns out to be "unsupported, documented gap."

## Steps

1. **Record the platform feasibility finding.** Write up, under `docs/next/` or
   `.local/prd/`: Linux `SO_PEERCRED` (getsockopt on the raw fd, already reachable
   via the `AsRawFd` import `src/ipc.rs:4` uses) and macOS `LOCAL_PEERCRED` /
   `getpeereid` are both implementable today with existing deps (`libc = "0.2"`,
   already a dependency). Windows is not a hard dead end: `windows-sys` is
   already a dependency with `Win32_System_Pipes` enabled
   (`Cargo.toml:49-66`), and `GetNamedPipeClientProcessId` + opening the peer
   process token (`OpenProcessToken`/`GetTokenInformation(TokenUser)`) to
   compare SIDs is a real mechanism — but it needs a new `Win32_Security`
   feature flag (not currently enabled) and meaningfully more code than the
   unix syscall pair. Recommend: ship unix first behind a
   `PlatformCapabilities`-style flag, decide Windows scope (implement now vs.
   file as a tracked gap) in this note rather than silently deferring it.

2. **Implement the peer-uid check (priority, independently shippable).** Add a
   platform function returning the connecting peer's uid (Linux: `SO_PEERCRED`;
   macOS: `getpeereid`), reject the connection when it differs from
   `std::process::id()`'s owning uid, on both accept paths:
   `src/api/server.rs` (`start_server_with_capabilities`, :101-144) and
   `src/server/client_accept.rs` (`accept_pending_client_connections_with_limit`,
   :45-95). Land the Windows decision from step 1 alongside — either a real
   check or an explicit no-op with the same shape as
   `restrict_socket_permissions`'s existing Windows stub (`src/ipc.rs:294-297`),
   never a silent skip.
   - Gate: `just test-one peer_cred` green; a test asserts a connection from a
     different uid (simulate via a spawned subprocess with dropped
     privileges, or a mock trait if uid-switching isn't available in CI) is
     rejected before any request is dispatched.

3. **Design the method capability tiering.** Using the full `Method` enum
   inventory (`src/api/schema.rs:45-238`, 91 variants), classify every
   client-facing method (exclude the three `#[serde(skip)]` internal
   `PaneGraphicsStream*` variants) into read-only / mutate / exec tiers. Write
   the tiering table as spec content in this proposal's `design.md` or inline
   here — no dispatch-site enforcement code in this step, this is design only.
   Explicitly resolve the ambiguous cases: `pane.report_agent` /
   `pane.report_agent_session` / `pane.report_metadata` /
   `pane.clear_agent_authority` (state-write, not process-exec — mutate, not
   exec); `integration.install`/`uninstall` and `plugin.link`/`enable`/`disable`
   (config-wiring mutate, adjacent to but not equal to exec); `agent.send_keys`
   / `agent.prompt` / `pane.send_text` / `pane.send_keys` / `pane.send_input`
   (exec tier — raw input to a live process is equivalent to arbitrary command
   execution); `server.live_handoff` / `server.stop` (exec/admin tier).

4. **Reconcile with the sibling spikes' outcomes.** Cross-check the tiering
   against whatever `spike-plugin-install-api` lands for `plugin.link` /
   `plugin.enable` (consent gate or audit event) and whatever
   `harden-live-handoff` already shipped for `server.live_handoff`
   (`c000681f`'s unix-only token gate) — the tiering model should classify both
   consistently with their actual enforcement, not just in the abstract. Note
   any mismatch as a follow-up rather than silently absorbing it here.

5. **Verify and close.** `just check` passes for the peer-uid check code (step
   2); tiering doc (steps 3-4) needs no build gate. done-when: proposal
   `spike-socket-auth-capability` archived with the peer-uid check landed (or
   explicitly scoped out with reasons) and the tiering table recorded.

## STOP conditions

- If uid-based peer rejection would break an existing legitimate cross-uid
  caller (e.g. a root-owned supervisor process managing a non-root herdr
  server, or vice versa), capture that case and design an explicit allowlist
  rather than dropping the check to accommodate it.
- If the Windows token/SID comparison can't distinguish "same interactive
  session, different process" from "genuinely different user" without pulling
  in session-enumeration APIs not already in the dependency tree, stop and
  record it as a documented Windows gap (matching `restrict_socket_permissions`'s
  existing precedent) rather than shipping a check that only looks real.
- If the tiering design (step 3) reveals that more than a handful of methods
  need per-call authorization state beyond a static tier (e.g. a capability
  that's read-only for one workspace but mutate for another), stop — that's a
  bigger model than this spike scoped, report before designing further.
