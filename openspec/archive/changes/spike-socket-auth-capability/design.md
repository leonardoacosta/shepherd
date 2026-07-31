# Design: socket-level auth / capability tiering

Covers Steps 1, 3, 4 of `tasks.md`. Step 2 (the peer-uid check implementation)
is being built concurrently in this same tree by another agent and is out of
scope for this document.

## Section A — Platform feasibility finding (Step 1)

**Linux.** `SO_PEERCRED` via `getsockopt` on the raw fd is implementable today.
`src/ipc.rs:3-4` already imports `std::os::fd::AsRawFd` under `#[cfg(unix)]`,
and `src/ipc.rs:159-188` (`probe_stream_closed`, unix arm) already extracts the
raw fd from `LocalStream::UdSocket` via `stream.inner().as_raw_fd()` and calls
into `libc` directly (`libc::recv`). The same `as_raw_fd()` handle is what a
`getsockopt(fd, SOL_SOCKET, SO_PEERCRED, ...)` call needs. `libc = "0.2"` is
already a dependency (`Cargo.toml:30`) and already exposes `libc::ucred` /
`SO_PEERCRED` on Linux targets — no new dependency.

**macOS.** `LOCAL_PEERCRED` / `getpeereid` is the equivalent primitive, same
shape: raw fd from `AsRawFd`, one `libc` call, no new dependency. macOS-specific
platform code already has a home at `src/platform/macos.rs`
(`src/platform/mod.rs:191-194` conditionally includes it), consistent with the
"platform code is isolated" rule in `CLAUDE.md`.

**Windows.** Not a hard dead end, but meaningfully more code and one missing
feature flag. `windows-sys` is already a dependency
(`Cargo.toml:50-67`) with `Win32_System_Pipes` enabled (`Cargo.toml:61`) — the
named-pipe primitives `probe_stream_closed`'s Windows arm already uses
(`src/ipc.rs:190-220`, `PeekNamedPipe`) confirm the pipe handle
(`AsHandle`/`AsRawHandle` on `LocalStream::NamedPipe`) is reachable the same
way a peer-identity check would need. The real mechanism is
`GetNamedPipeClientProcessId` (needs the process id of the connecting client)
followed by opening that process's token (`OpenProcessToken`) and comparing
its user SID (`GetTokenInformation(TokenUser)`) against the server's own SID.
**Verified**: `Cargo.toml:50-67` does NOT currently enable a `Win32_Security`
feature — none of the 16 listed features (`Wdk_System_Threading` through
`Win32_UI_WindowsAndMessaging`) is `Win32_Security` or a
`Win32_Security_*` sub-feature. Enabling it is a one-line `Cargo.toml` change,
but the SID-comparison call sequence (open process handle -> open token ->
query token -> extract + compare SID) is several times more code than the two
Unix syscalls, and pulls in a security-sensitive Win32 surface this codebase
doesn't touch anywhere else today.

**Recorded decision (not re-litigated here):** ship the peer-uid check
Unix-first behind a `PlatformCapabilities`-style flag
(`src/platform/mod.rs:45-66`, the existing `live_handoff` / `remote_attach` /
`direct_terminal_attach` boolean fields set via `cfg!(unix)` /
`cfg!(target_os = "macos")` is the pattern to extend — e.g. a new
`peer_credential_check: bool` field). Windows is a **tracked, documented gap**,
not a silent deferral — the same shape as `restrict_socket_permissions`'s
existing Windows no-op (`src/ipc.rs:358-361`) and the precedent set by
`c000681f` ("fix: gate handoff import token on unix"), which gated
`handoff_import_token` to `#[cfg(unix)]` and made the Windows arm return a hard
`io::Error::other("live handoff is only supported on Unix")`
(`src/server/headless.rs:3008-3021`) instead of attempting a partial or
silently-skipped check.

**STOP-condition reasoning that drove the Unix-first call:** the
token/SID comparison can tell you "same Windows user account" but cannot
cleanly distinguish "same interactive session, different (possibly untrusted)
process" from "genuinely different user" — that finer distinction needs
session-enumeration APIs (e.g. `WTSEnumerateSessions` / querying the session
ID a token belongs to) that are outside the current dependency tree and outside
what `Win32_System_Pipes` + a hypothetical `Win32_Security` feature flag would
provide on their own. Per the spike's own STOP condition, a check that only
*looks* real (same-user-account gate, no session boundary) is worse than an
explicit documented gap, so Windows peer verification is scoped out rather than
shipped in a weaker form.

## Section B — Method capability tiering (Step 3)

Design only — **no dispatch-site enforcement in this spike**. This tiers all
client-facing `Method` variants (`src/api/schema.rs:45-238`) into
read-only / mutate / exec so a future enforcement layer has a model to consult.

**Inventory count:** the enum has 94 variants total. Three are excluded as
internal, not client-facing: `PaneGraphicsStreamSet`, `PaneGraphicsStreamOpen`,
`PaneGraphicsStreamClose` (`src/api/schema.rs:186-191`), each carrying both
`#[serde(skip)]` and `#[schemars(skip)]` — they exist only as internal
re-dispatch targets for the streaming graphics machinery
(`src/api/server/pane_graphics_stream.rs`), never appear on the wire, and have
no `#[serde(rename = "...")]` wire name. That leaves **91 client-facing
variants**, matching the count `tasks.md` names. Note:
`PaneGraphicsStream` itself (`src/api/schema.rs:180-182`, wire name
`pane.graphics.stream`) carries only `#[schemars(skip)]` — it IS dispatchable
and IS client-facing (excluded from generated JSON-schema docs only, not from
serde); it is tiered below and not counted among the three excluded variants.

| Wire name | Variant | Tier | Note |
| --- | --- | --- | --- |
| `ping` | Ping | read-only | |
| `server.stop` | ServerStop | exec/admin | calibration anchor |
| `server.live_handoff` | ServerLiveHandoff | exec/admin | calibration anchor; see Section C cross-check |
| `server.reload_config` | ServerReloadConfig | mutate | reloads server-wide config state |
| `server.agent_manifests` | ServerAgentManifests | read-only | |
| `server.reload_agent_manifests` | ServerReloadAgentManifests | mutate | reloads detection rules from disk, not a process exec |
| `notification.show` | NotificationShow | mutate | client-side UI side effect |
| `client.window_title.set` | ClientWindowTitleSet | mutate | |
| `client.window_title.clear` | ClientWindowTitleClear | mutate | |
| `session.snapshot` | SessionSnapshot | read-only | |
| `workspace.create` | WorkspaceCreate | mutate | `WorkspaceCreateParams` has no `command` field — spawns the pane's default shell only, not arbitrary exec |
| `workspace.list` | WorkspaceList | read-only | |
| `workspace.get` | WorkspaceGet | read-only | |
| `workspace.focus` | WorkspaceFocus | mutate | |
| `workspace.rename` | WorkspaceRename | mutate | |
| `workspace.move` | WorkspaceMove | mutate | |
| `workspace.move_block` | WorkspaceMoveBlock | mutate | |
| `workspace.report_metadata` | WorkspaceReportMetadata | mutate | state-write, same class as the `pane.report_*` anchors |
| `workspace.close` | WorkspaceClose | mutate | |
| `worktree.list` | WorktreeList | read-only | |
| `worktree.create` | WorktreeCreate | mutate | runs a fixed `git worktree add`-class operation, not an arbitrary command — filesystem/git mutation, not open-ended exec |
| `worktree.open` | WorktreeOpen | mutate | attaches/opens an existing worktree, changes session state |
| `worktree.remove` | WorktreeRemove | mutate | filesystem mutation |
| `tab.create` | TabCreate | mutate | no `command` field, same reasoning as `workspace.create` |
| `tab.list` | TabList | read-only | |
| `tab.get` | TabGet | read-only | |
| `tab.focus` | TabFocus | mutate | |
| `tab.rename` | TabRename | mutate | |
| `tab.move` | TabMove | mutate | |
| `tab.close` | TabClose | mutate | |
| `agent.list` | AgentList | read-only | |
| `agent.get` | AgentGet | read-only | |
| `agent.read` | AgentRead | read-only | |
| `agent.explain` | AgentExplain | read-only | |
| `agent.send_keys` | AgentSendKeys | exec | calibration anchor |
| `agent.rename` | AgentRename | mutate | |
| `agent.view.set` | AgentViewSet | mutate | |
| `agent.view.clear` | AgentViewClear | mutate | |
| `agent.focus` | AgentFocus | mutate | |
| `agent.start` | AgentStart | exec | `AgentStartParams.args: Vec<String>` is attacker-controllable process argv — spawns a new process, same risk class as raw input |
| `agent.prompt` | AgentPrompt | exec | calibration anchor |
| `agent.wait` | AgentWait | read-only | blocking observer, no mutation |
| `pane.split` | PaneSplit | mutate | no `command` field (only `cwd`/`env`) — spawns the pane's default shell, not arbitrary exec; still a process spawn, flagged for future review |
| `pane.swap` | PaneSwap | mutate | |
| `pane.move` | PaneMove | mutate | |
| `pane.zoom` | PaneZoom | mutate | |
| `pane.layout` | PaneLayout | mutate | |
| `pane.process_info` | PaneProcessInfo | read-only | |
| `layout.export` | LayoutExport | read-only | |
| `layout.apply` | LayoutApply | **exec** | **non-obvious**: `LayoutApplyParams.root: LayoutNode` recurses into `LayoutPane.command: Option<Vec<String>>` (`src/api/schema/panes.rs:168-179`) — this method can launch an arbitrary command with arbitrary args per pane, same risk class as `agent.start`, higher blast radius (whole layout tree) |
| `layout.set_split_ratio` | LayoutSetSplitRatio | mutate | |
| `pane.neighbor` | PaneNeighbor | read-only | |
| `pane.edges` | PaneEdges | read-only | |
| `pane.focus_direction` | PaneFocusDirection | mutate | |
| `pane.resize` | PaneResize | mutate | |
| `pane.list` | PaneList | read-only | |
| `pane.current` | PaneCurrent | read-only | |
| `pane.get` | PaneGet | read-only | |
| `pane.focus` | PaneFocus | mutate | |
| `pane.rename` | PaneRename | mutate | |
| `pane.send_text` | PaneSendText | exec | calibration anchor |
| `pane.send_keys` | PaneSendKeys | exec | calibration anchor |
| `pane.send_input` | PaneSendInput | exec | calibration anchor |
| `pane.read` | PaneRead | read-only | |
| `pane.graphics.set` | PaneGraphicsSet | mutate | |
| `pane.graphics.clear` | PaneGraphicsClear | mutate | |
| `pane.graphics.info` | PaneGraphicsInfo | read-only | |
| `pane.graphics.stream` | PaneGraphicsStream | mutate | acquires exclusive stream ownership (`owner` field) for a pane's graphics channel, can preempt an existing owner — not a pure read |
| `pane.report_agent` | PaneReportAgent | mutate | calibration anchor |
| `pane.report_agent_session` | PaneReportAgentSession | mutate | calibration anchor |
| `pane.report_metadata` | PaneReportMetadata | mutate | calibration anchor |
| `pane.clear_agent_authority` | PaneClearAgentAuthority | mutate | calibration anchor |
| `pane.release_agent` | PaneReleaseAgent | mutate | |
| `pane.close` | PaneClose | mutate | |
| `popup.close` | PopupClose | mutate | |
| `events.subscribe` | EventsSubscribe | read-only | |
| `events.wait` | EventsWait | read-only | |
| `pane.wait_for_output` | PaneWaitForOutput | read-only | |
| `integration.install` | IntegrationInstall | mutate | calibration anchor |
| `integration.uninstall` | IntegrationUninstall | mutate | calibration anchor |
| `plugin.link` | PluginLink | mutate | calibration anchor; see Section C mismatch note |
| `plugin.list` | PluginList | read-only | |
| `plugin.unlink` | PluginUnlink | mutate | |
| `plugin.enable` | PluginEnable | mutate | calibration anchor; see Section C mismatch note |
| `plugin.disable` | PluginDisable | mutate | calibration anchor |
| `plugin.action.list` | PluginActionList | read-only | |
| `plugin.action.invoke` | PluginActionInvoke | mutate | invokes a declared action of an already-linked, already-consented-to plugin — bounded by the linking trust gate, not a fresh arbitrary command |
| `plugin.log.list` | PluginLogList | read-only | |
| `plugin.pane.open` | PluginPaneOpen | mutate | opens a plugin-declared UI entrypoint, same trust-boundary reasoning as `plugin.action.invoke` |
| `plugin.pane.focus` | PluginPaneFocus | mutate | |
| `plugin.pane.close` | PluginPaneClose | mutate | |

**Tier totals:** read-only = 28, mutate = 54, exec = 9. Total = 91 (matches the
client-facing count above). Excluded (internal, not tiered): 3.

**STOP condition check (did NOT fire):** none of the 91 methods needs per-call
authorization state beyond its static tier — no method's correct tier depends
on which workspace/tab/pane it targets, the caller's identity beyond
same-uid, or any other runtime condition. Every method maps to exactly one
fixed tier. The two closest candidates were examined explicitly:
`layout.apply` (tiered exec outright, not conditionally, because the
`command` field is always present in the schema even when a given payload
happens not to use it) and `plugin.action.invoke` (tiered mutate outright,
scoped by the plugin's own link-time consent rather than by a per-call
condition). Neither needs a dynamic model. This spike's static-tier design is
sufficient; the bigger per-call-authorization model described in the STOP
condition is not warranted.

## Section C — Reconciliation with sibling work (Step 4)

**`spike-plugin-install-api` (open proposal).** Its `proposal.md` and
`tasks.md` document a real, currently-unclosed consent gap: `plugin.link` /
`plugin.enable` have no confirmation step today, so any same-uid caller can
link + enable a plugin whose `[[startup]]` hooks then execute on every server
start, with no preview and no prompt — mirroring the CLI's own
`plugin_install` -> preview -> `confirm()` flow (`src/cli/plugin.rs:154-260`)
that only exists for the CLI's install path, not the API's link/enable path.

**Mismatch, flagged rather than silently absorbed:** this spike's tiering
treats `plugin.link` / `plugin.enable` as ordinary `mutate` — the same tier as,
e.g., `tab.rename` or `workspace.close`. But the sibling spike's own threat
model argues these two methods carry materially higher risk than a typical
mutate call, because they gate *future* arbitrary code execution (the startup
hooks) rather than making an immediate bounded state change. A flat `mutate`
tier has no slot for "this mutate call needs a confirmation gate before it's
allowed to proceed" — that is a orthogonal, per-method property this spike's
model does not represent. **Follow-up recommendation:** when
`spike-plugin-install-api` lands its consent mechanism (foreground
confirmation modal or non-suppressible audit event), it should be modeled as a
tier *modifier* (e.g. a `requires_confirmation: bool` alongside the tier, only
set on `plugin.link` / `plugin.enable` today) rather than either inventing a
fourth tier or silently treating the confirmation as orthogonal to tiering.
This spike does not add that modifier itself — it is out of scope here — but
records the gap so the enforcement design doesn't have to rediscover it.

**`harden-live-handoff` (already shipped, commit `c000681f`).** Read via
`git show c000681f` and the current `src/server/headless.rs:3008-3021`: the
fix split `handoff_import_token()` into a `#[cfg(unix)]` arm that reads
`HANDOFF_TOKEN_ENV_VAR` and a `#[cfg(not(unix))]` arm that returns a hard
`io::Error::other("live handoff is only supported on Unix")` — no partial or
silently-degraded Windows behavior. **Cross-check result: consistent, no
mismatch.** `server.live_handoff`'s exec/admin tier here is exactly the class
of method that should carry the strictest gating available, and the shipped
enforcement already does that (hard Unix-only token requirement, hard failure
elsewhere) rather than a softer per-platform fallback. This also validates the
Unix-first sequencing recorded in Section A: the peer-uid check and the
handoff token gate are both unix-only for the same underlying reason
(no equivalent-strength primitive is cheaply available on Windows today), so
the two hardening efforts are directionally aligned rather than diverging.

## Verification note

No Rust code changes accompany this document — it is design content only, per
the task contract. `just check` is unaffected by this file.
