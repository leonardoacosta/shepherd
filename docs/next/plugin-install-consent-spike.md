# Plugin Install / Consent Spike

Date: 2026-07-31
Status: decisions recorded; implementation follows as tracked work
Scope: design spike for the plugin install surface and the `plugin.link` / `plugin.enable` consent gap

## The asymmetry

Install and link are gated very differently.

`herdr plugin install` runs a deliberate trust flow in the CLI: `plugin_install`
(`src/cli/plugin.rs:448`) clones the source, loads and previews the manifest
(`:490-493`), checks replacement rules (`:495`), and only then prompts through
`confirm("Install this plugin?")` (`:500`, helper at `:1844`). After the prompt it calls
`register_installed_plugin` (`:1208`), which builds `Method::PluginLink` and sends it over
the ordinary JSON API socket (`:1220` → `src/cli.rs:743` → `src/api/client.rs:38`).

The API has no equivalent. `src/api/schema.rs:216-238` exposes `plugin.link`, `list`,
`unlink`, `enable`/`disable`, `action.list`/`invoke`, `log.list`, and
`pane.open|focus|close` — but no `plugin.install`, and no confirmation step anywhere. The
mouse-first TUI has no plugin install surface at all, while
`website/src/content/docs/marketplace.mdx` documents a browse-and-install story whose
install step forces the user out to a shell.

## Threat model

The attack is a **single API call**, not the two-step link-then-enable the proposal
describes.

`PluginLinkParams.enabled` defaults to `true` (`src/api/schema/plugins.rs:14-15`, via
`default_true`). So `plugin.link {"path": "/tmp/attacker-manifest"}` links *and* enables in
one request. `handle_plugin_link` (`src/app/api/plugins/mod.rs:68`) loads the manifest,
creates the plugin's user dirs, and persists it to the registry — with no confirmation of
any kind.

The payoff lands at the next server start. `run_plugin_startup_hooks`
(`src/app/api/plugins/runtime.rs:183`) selects every plugin where
`plugin.enabled && plugin_manifest_available(plugin) && !plugin.startup.is_empty()` (`:191`)
and spawns each `[[startup]]` command via `start_plugin_command` (`:206` → `:16`), using
`command_for_argv_in_dir` with cwd set to the plugin's own root (`:122`). The child runs
with the full privileges of the herdr server process, unsandboxed, and its environment
includes `HERDR_SOCKET_PATH` / `HERDR_BIN_PATH` / `HERDR_ENV=1` — so the spawned process can
turn around and drive the API itself. `headless.rs:2967` and `:3085` call
`run_plugin_startup_hooks` unconditionally right after server start.

Enablement alone is sufficient. Nothing re-confirms at execution time.

### What the peer-uid check does not solve

The peer-credential check shipped in `80e25966` (`peer_uid_authorized`, `src/ipc.rs:228`,
wired at `src/api/server.rs:118`) rejects connections whose peer uid differs from the
server's. That is a real boundary, but it is **orthogonal to this threat**: herdr's whole
purpose is hosting coding agents that run as the same uid as the server. A hostile or
compromised agent in a pane is precisely a same-uid peer, so it passes the check
unconditionally. On Windows the check is a documented no-op (`src/ipc.rs:246`,
`src/platform/mod.rs:51-53`), so there is no peer gate there at all.

### The CLI gate is already bypassable by the same attacker

The CLI trust flow is not a security boundary against a same-uid process, for two
independent reasons:

1. **The API path skips it entirely** — the single `plugin.link` call above.
2. **The CLI itself can be driven non-interactively.** `--yes` (`src/cli/plugin.rs:471-474`)
   skips `confirm()` and nothing else; every other step — checkout, manifest load,
   `run_plugin_build_commands` (`:504`, which already executes the plugin's own build
   commands), post-build re-validation, and the final `PluginLink` — runs identically. The
   guard at `:482` only *requires* `--yes` when stdin is not a TTY; it does not restrict who
   may pass it. An agent with shell access can simply run
   `herdr plugin install <src> --yes`.

This matters for scoping: closing the API gap alone does **not** close the threat. It closes
one of two doors.

## Why the server cannot tell CLI from agent

`Request` (`src/api/schema.rs:33-36`) is `{ id: String, method: Method }`. There is no
caller name, no client identity, no handshake metadata on the API socket. The CLI sends
`id: "cli:plugin"` (`src/cli/plugin.rs:1213`), but that string is caller-supplied and
unauthenticated — any process can send the identical envelope, and nothing in the codebase
ties `Request.id` to an authorization decision.

The CLI uses the *same* public API socket as any other client (`ApiClient::local()`,
`src/api/client.rs:24,38`). There is no privileged transport.

This has a consequence that constrains every option below: **any pre-consent proof the CLI
could present, a same-uid attacker could also obtain.** Same uid means same filesystem
access, same environment, same ability to read whatever token or socket the CLI reads. A
shared secret does not create a boundary between two processes running as the same user.
Only something requiring a *human* — a TTY prompt, a UI the attacker cannot drive — creates
one.

## Options

### 1. Foreground-client confirmation modal

The proposal suggests routing API-originated link/enable through a TUI modal, reusing the
`src/ui/release_notes.rs` overlay machinery.

There is no plumbing for this. The client protocol has **no server→client
request/response precedent anywhere**: every `ServerMessage` variant
(`src/protocol/wire.rs:675-763` — `Welcome`, `Frame`, `FrameDelta`, `Terminal`, `Graphics`,
`ServerShutdown`, `Notify`, `Clipboard`, `WindowTitle`, `ReloadSoundConfig`, `MouseCapture`,
`KittyKeyboardReportAll`, `PrefixInputSource`) is a one-way push carrying no request id the
client is expected to answer, and every `ClientMessage` variant (`:320-410`) is
client-initiated. The transport underneath (`src/server/client_transport.rs`) is a
fire-and-forget byte queue. Building this means new wire variants with correlation ids, plus
a way for the plugin handler to await a reply *without stalling the single-threaded app
loop* it runs on (`src/app/mod.rs:1110`).

It also conflicts with the runtime/client boundary guardrail in `CLAUDE.md`: it would make a
server-side security control depend on the private TUI client socket. And it has no answer
for headless mode (`src/server/headless.rs:228`), where no TUI client may ever attach — the
control would have to fail closed, which is a policy decision in its own right.

Not recommended as the first move. Large, and it puts a security property on the surface the
repo is actively migrating away from.

### 2. Non-suppressible audit event + toast

There is no plugin event today: `EventKind` (`src/api/schema/events.rs:204-231`) has no
`PluginLinked` / `PluginEnabled` variant, and neither does `PLUGIN_HOOK_EVENT_KINDS`
(`:296`). So this is new work, not a wire-up.

More importantly it cannot be made non-suppressible with the current design.
`EventsSubscribeParams { subscriptions }` (`events.rs:12`) is an explicit opt-in list and
there is no always-delivered broadcast tier. A client that does not subscribe never sees the
event. And it is detection, not prevention — the startup hook still runs.

Worth doing as defence in depth. Not sufficient as the fix.

### 3. Deny-by-default for raw API link/enable

Refuse to *enable* a plugin that declares `[[startup]]` commands when the request arrives
raw over the API, and require the human-gated path for that transition.

The choke point is clean. Every caller — CLI, raw socket client, anything — funnels through
the single-threaded app loop (`src/app/mod.rs:1110` → `src/app/api.rs:889`) before reaching
`handle_plugin_link` (`src/app/api/plugins/mod.rs:68`) or `set_plugin_enabled` (`:612`). A
check at the top of those two functions sees 100% of callers with no bypass, and sits before
any filesystem mutation.

The honest limitation, per the section above: this cannot distinguish "CLI" from "agent" by
identity, because no such distinction is achievable between same-uid processes. What it
*can* do is distinguish **capability**: allow `plugin.link` to register a plugin as
disabled, and treat "enable a plugin that declares startup commands" as the privileged
transition that requires an out-of-band human act. That degrades the attack from "one API
call to arbitrary code execution at next start" to "attacker must also get a human to
approve" — which is the actual security property the CLI prompt was trying to provide.

`--yes` keeps working: it is a human at a shell deliberately electing to pre-consent. The
STOP condition is satisfied without dropping the gate. Note that this also means `--yes`
remains an agent-reachable bypass (see above) — a separate follow-up, not something this
gate can close.

## Recommendation

**Take option 3 as the fix, with option 2 layered on as an audit trail. Do not build option
1 in this spike.**

Concretely:

- Flip the risky default: `PluginLinkParams.enabled` defaulting to `true`
  (`src/api/schema/plugins.rs:14-15`) is what makes the attack a single call. This is a
  breaking API change and needs a maintainer decision.
- Gate the privileged transition at `handle_plugin_link` / `set_plugin_enabled` for plugins
  declaring `[[startup]]` commands.
- Add a `PluginLinked` / `PluginEnabled` `EventKind` so the action is at least observable.
- File the `--yes` agent-reachable bypass as its own follow-up. It is a real hole and it is
  out of scope here.

## Install location (Step 3)

**Decided: install stays CLI-only.** `marketplace.mdx` deliberately frames the index as "not
a reviewed catalog", which is an argument for install staying a deliberate, out-of-band act;
and the confirmation mechanism a TUI `plugin.install` would need is exactly the option-1
plumbing that does not exist yet. Revisit once there is a server→client request/response
primitive, which would serve other features too.

## Decisions and follow-up work

Recorded 2026-07-31. No code landed with this spike — the spike's acceptance allows the
consent gap to be filed as an immediate follow-up with the mechanism specified, which is what
this section does.

1. **Flip `PluginLinkParams.enabled` to default `false`** (`src/api/schema/plugins.rs:14-15`).
   Approved as a breaking API change. An API caller omitting `enabled` must link the plugin
   disabled. `register_installed_plugin` (`src/cli/plugin.rs:1208`) must pass `enabled: true`
   explicitly so `herdr plugin install` keeps its current end state, and every other in-repo
   `PluginLinkParams` construction site must be made explicit. The generated TypeScript
   client types (`clients/ts`, checked by `just generated-api-client-test`) need regenerating
   if the emitted schema changes. Needs a test asserting a `plugin.link` omitting `enabled`
   yields a plugin that is not enabled.

   Note the limit honestly: this removes the *one-call* path but does not close the gap. An
   attacker can still pass `enabled: true` explicitly and get startup-hook execution at the
   next server start. It reduces accidental exposure, not deliberate abuse.

2. **Gate the privileged transition** — refuse to enable a plugin declaring `[[startup]]`
   commands when the request arrives raw over the API, at `handle_plugin_link`
   (`src/app/api/plugins/mod.rs:68`) and `set_plugin_enabled` (`:612`). This is the actual
   fix for the threat in this note; item 1 alone is not sufficient. Deliberately scoped out
   of the first change.

3. **Add a `PluginLinked` / `PluginEnabled` `EventKind`** (`src/api/schema/events.rs:204-231`)
   so the action is observable. Defence in depth only — it is opt-in by subscription
   (`events.rs:12`) and therefore suppressible, and it does not prevent execution.

4. **The `--yes` bypass is a separate hole.** `herdr plugin install <src> --yes` is reachable
   by any same-uid process, including an agent in a pane, and skips `confirm()` entirely
   (`src/cli/plugin.rs:471-474`, `:500`). No API-side gate closes this. Tracked as its own
   follow-up.
