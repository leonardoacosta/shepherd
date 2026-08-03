# Design spike: let the remote catalog add agents, not just update the compiled-in ones

Base commit: `c000681f` · Route: proposal (design spike) · Effort: L (full) / M (community-PR path) · Confidence: MED (constraint is definite; loosening it is a maintainer call) · Category: direction

> Re-scoped 2026-07-30 against `c000681f` (originally written against `1de05dc2`).
> Bundled-manifest storage had already moved from an `Agent`-enum-keyed match to a
> string-keyed lookup array (`BUNDLED_MANIFESTS`) between those two commits. That
> changed the shape of the "string-id path" option below, so this file's premise,
> citations, and the coupling inventory were redone against current source. The
> spike's conclusion is unchanged: the remote catalog still cannot introduce a new
> agent, but the reason has shifted from "everything is enum-keyed" to "storage is
> string-keyed, resolution is not" — see Coupling Inventory.

## Why

Resolves advisory finding DIRECTION-04 (audit against `1de05dc2`).

Recognizing a new coding agent is data (a manifest), but the remote catalog
(`https://shepherd.dev/agent-detection/index.toml`) can only ever **replace** a
manifest for an agent the binary already knows, never add a new one. The
constraint is real, but it is enforced at a different seam than the original
draft claimed:

- `src/detect/manifest.rs:240-258` — `BUNDLED_MANIFESTS` is a
  `const &[(&str, &str)]` of `include_str!` entries keyed by **string id**, not
  the `Agent` enum. Lookup at `src/detect/manifest.rs:718-722` is
  `.find(|(manifest_id, _)| *manifest_id == id)`. (This is the part of the
  original draft that was wrong: it cited this as still enum-matched.)
- The enum survives everywhere around that storage change. The decisive gate is
  `parse_catalog()` in `src/detect/manifest_update.rs:311-341`: for every entry
  in the remote index it calls `parse_agent_label(&entry.id)`
  (`src/detect/mod.rs:167`), and on `None` it logs `"skipping unknown remote
  manifest agent"` and **drops the entry** (`src/detect/manifest_update.rs:325`).
  An agent id the binary has never heard of never gets past this line, no
  matter what the catalog or the manifest content says.
- `scripts/agent_detection_manifest_check.py:270-283` (`validate_catalog`)
  independently enforces the same constraint at CI time: `if agent_id not in
  bundled: raise CheckError(f"... unknown agent {agent_id}; binary cannot
  identify it")`. The website catalog cannot list an agent that isn't already a
  bundled manifest file, full stop.
- Adding a 20th agent still requires touching `src/detect/mod.rs` (the `Agent`
  enum + `identify_agent` + `parse_agent_label`/`lookup_agent` match arms),
  `src/detect/manifests/` (a new bundled `.toml` + `BUNDLED_MANIFESTS` entry),
  `src/config/sound.rs` (per-agent sound match), `src/config/sidebar.rs`
  (per-agent sidebar-row match), and `src/cli/spec.rs` (`Agent::ALL` listing),
  then waiting for a release. `CONTRIBUTING.md:41` routes such a feature
  through a Discussion + maintainer approval first.

New agents ship constantly, and "shepherd recognizes my agent" is the single
highest-value outside contribution — it is data, not code. The out-of-band
delivery infra already exists and is tested (`src/detect/manifest/tests.rs`
covers remote/local/bundled precedence and version shadowing); only the
resolution boundary (`parse_agent_label` / `Agent` enum) stands in the way, not
the bundled-storage representation.

## Coupling Inventory

Every site that is `Agent`-enum-typed today, versus every site already keyed
by string id, as of `c000681f`. "Conversion seam" marks where today's code
crosses between the two representations — this is what a string-id path would
have to move or delete, not just extend.

### Enum-typed (`Agent`, `Option<Agent>`) — the blast radius

| Site | What it does | Citation |
| --- | --- | --- |
| `Agent` enum + variants | 21-variant identity type | `src/detect/mod.rs:43-65` |
| `Agent::ALL` | Full variant list, drives CLI listing/completion | `src/detect/mod.rs:68` (21 entries), consumed at `src/cli/spec.rs:451,1263` |
| `Agent::SCREEN_MANIFEST_AGENTS` | Subset (19) that has a bundled screen-detection manifest; drives `ManifestCache` construction | `src/detect/mod.rs:92-111` |
| `lookup_agent` (:177) / `parse_canonical_agent_label` (:172) / `parse_agent_label` (:167) | string -> `Option<Agent>`, alias table | `src/detect/mod.rs:167-204` |
| `agent_label(agent: Agent) -> &'static str` | `Agent` -> canonical string, match over all variants | `src/detect/mod.rs:115-139` |
| `identify_agent` (:206) / `identify_agent_in_job` (:210) | process name -> `Option<Agent>` (via `parse_agent_label`), foreground-job scoring | `src/detect/mod.rs:206-247` |
| `ManifestCache` | `Vec<(Agent, Option<LoadedManifest>)>`, built by iterating `SCREEN_MANIFEST_AGENTS` | `src/detect/manifest.rs:135-137,290-296` |
| `bundled_manifest(agent: Agent)` | Converts `Agent` -> string via `agent_label`, then string-searches `BUNDLED_MANIFESTS` | `src/detect/manifest.rs:716-724` |
| `load_manifest_uncached(agent: Agent)` (:577), `override_path` (:1107), `read_remote_manifest` (:732) | Manifest load/merge pipeline, all `Agent`-parameterized | `src/detect/manifest.rs:577,732,1107` |
| `manifest_matches_agent(manifest, agent: Agent)` | Confirms a manifest's own `id`/`aliases` resolve (via `parse_agent_label`) to the expected enum value | `src/detect/manifest.rs:1119-1128` |
| `parse_remote_manifest_for_agent(agent: Agent, content)` | Requires a pre-known `Agent` before it will even parse remote content for that agent | `src/detect/manifest.rs:875-901` |
| `remote_manifest_path`, `cached_remote_version`, `commit_remote_manifest` | All `Agent`-parameterized; decide the on-disk cache path per agent | `src/detect/manifest_update.rs:379-...`, `385`, `392` |
| `parse_catalog` (remote index ingestion) | **The decisive gate** — drops any catalog entry whose id doesn't already resolve via `parse_agent_label` | `src/detect/manifest_update.rs:311-341` |
| `src/config/sound.rs::for_agent` | 20-arm match, per-agent sound setting | `src/config/sound.rs:120-143` |
| `src/config/sidebar.rs::rows_for_agent` + `Agent::ALL`-derived test fixtures | Per-agent sidebar row/token config | `src/config/sidebar.rs:383`, `604-624` |
| `TerminalState.detected_agent: Option<Agent>`, `effective_known_agent()` | Core per-pane detected-agent field (screen-detection result) | `src/terminal/state.rs:1711-1713` and struct field |
| `src/pane.rs` | Process-based agent identification/lifecycle: `current_agent`, `suppressed_agent`, `foreground_shell_agent_action`, `begin_graceful_release(agent: Agent)` — the deepest, most enum-native subsystem in the codebase | `src/pane.rs:137-544, 2407` (representative; ~35 enum sites total) |
| `src/platform/{macos,windows,mod}.rs` | Process-name -> `Agent` matching, platform-specific | `src/platform/macos.rs:1056`, `src/platform/mod.rs:405,409`, `src/platform/windows.rs:345` |
| `src/events.rs::AppEvent::StateChanged{agent: Option<Agent>}`, `HookAgentReleased{known_agent: Option<Agent>}` | Internal event bus carries the enum for the screen-detection path | `src/events.rs:62,114` |
| `src/app/*.rs`, `src/ui/sidebar.rs` | Mostly test fixtures and `NextAgent`/`PreviousAgent` nav ordering over the enum | `src/app/mod.rs:4063,4113,4820,4844`; `src/app/input/navigate.rs:1465-1466,1920`; `src/ui/sidebar.rs` (~15 sites, all tests) |

### Already string-keyed — no `Agent` enum in the loop

| Site | What it does | Citation |
| --- | --- | --- |
| `BUNDLED_MANIFESTS` | `const &[(&str, &str)]`, storage only — callers still pass in an `Agent` and convert | `src/detect/manifest.rs:240-258` |
| `HookAuthority.agent_label: String` + all `Hook*`/`AgentSessionReported` `AppEvent` variants | Hook-sourced agent identity, end-to-end string, never touches `Agent` | `src/terminal/state.rs:18-20` (struct), `src/events.rs:71-114` |
| `full_lifecycle_hook_authority(source: &str, agent_label: &str)`, `session_identity_only_integration` | Hardcoded closed allowlist of `(source, label)` string pairs (`shepherd:pi`/"pi", `shepherd:omp`/"omp", etc. — 6 pairs + 1) — string-typed, but just as closed as the enum, not a precedent for open string ids | `src/detect/mod.rs:283-297` |
| `src/api/schema/agents.rs` (`AgentInfo.agent`, `AgentSessionInfo.agent`), `src/api/schema/panes.rs`, `src/api/schema/events.rs`, `src/api/schema/plugins.rs::focused_pane_agent` | Wire/API schema — **already fully `String`/`Option<String>`**, no `Agent` enum on the wire. This is the other part of the original draft that was wrong: it cited `src/api/schema/agents.rs` as enum-coupled; it is not. | `src/api/schema/agents.rs:189,197,228` |
| `TerminalState::effective_agent_label() -> Option<&str>` | Public string accessor most consumers already use instead of the enum | `src/terminal/state.rs:1698-1709` |

### The conversion seam

Two functions are the entire `Agent` <-> `String` boundary today:
`agent_label(Agent) -> &str` and `parse_agent_label(&str) -> Option<Agent>`
(`src/detect/mod.rs:115, 167`). Every wire-schema site and every hook-event
site already lives on the string side of that boundary and never needs to
cross it. The one site that matters for this spike is `parse_catalog`
(`src/detect/manifest_update.rs:311-341`): it is the single place a remote
catalog entry gets converted from string to `Option<Agent>`, and it is where an
unrecognized agent id dies today. A string-id path does not need to touch the
wire schema or the hook-event path at all — both are already string-native.
It has to change what happens when `parse_catalog`'s conversion returns `None`,
and then thread that "unresolved but present" identity through the
screen-detection stack that stays enum-native below it: `ManifestCache`,
`bundled_manifest`/`load_manifest_uncached`, `remote_manifest_path` and
friends, `TerminalState.detected_agent`, `src/pane.rs`'s process-identification
state machine, `src/config/sound.rs`, `src/config/sidebar.rs`, and
`src/cli/spec.rs::Agent::ALL`. That is a smaller list than the original draft
assumed (the wire schema and hook path are free), but `src/pane.rs` and the
manifest-cache/resolution chain are still substantial, `Copy`-enum-shaped
surface.

## Trust controls already enforced on manifest content

A remotely-delivered manifest's `rules` are regex/gate trees
(`src/detect/manifest.rs` types), so a community submission is untrusted
pattern-matching input. Caps exist today at two independent layers:

- **Rust runtime** (`src/detect/manifest.rs:265-270`, enforced in
  `validate_manifest`/`validate_gate` at parse time —
  `src/detect/manifest.rs:903-1054`): `MAX_RULES_PER_MANIFEST = 128`,
  `MAX_GATE_DEPTH = 8`, `MAX_TOTAL_GATES = 512`, `MAX_MATCHERS_PER_GATE = 32`,
  `MAX_TOTAL_MATCHERS = 1024`, `MAX_MATCHER_CHARS = 512`. These apply to
  *any* manifest the binary parses — bundled, override, or remote — including
  a manifest fetched from the untrusted remote catalog today, before this
  spike changes anything.
- **CI script** (`scripts/agent_detection_manifest_check.py`) mirrors the same
  six constants and additionally hard-fails the website build if a catalog
  entry's `id` isn't already a bundled manifest (`validate_catalog`, line
  ~270-283) — a second, independent enforcement of the same constraint this
  spike is about.
- **Engine algorithmic guarantee**: manifests compile through `regex::Regex`
  (`src/detect/manifest.rs:7`), Rust's linear-time regex engine — it has no
  backtracking, so catastrophic-backtracking ReDoS via a malicious pattern is
  not a risk class here the way it would be for a backtracking engine (PCRE,
  Python `re`, etc.). The six caps above bound compile time/memory by rule
  count and matcher size, not by defending against exponential-time matching,
  because that failure mode doesn't exist for this regex engine.
- **What is not enforced**: nothing today validates the *content* of a regex
  beyond `MAX_MATCHER_CHARS` — a community manifest could still submit a
  pattern that matches too broadly (false idle/blocked detection) or encodes
  something misleading in `state_labels`/display text. That is a correctness
  and abuse-review problem, not a resource-exhaustion one, and would need a
  human-reviewed acceptance step (golden-fixture test) regardless of which
  path below is chosen.

## What this spike produces

A decision between two grounded options, and the design for the chosen one.

**Recommendation: community-PR path.** The full string-id path would need to
touch the `Agent` enum's core resolution machinery (`parse_agent_label`,
`agent_label`, `identify_agent`) plus the still-enum-native
screen-detection/manifest-cache chain and `src/pane.rs`'s process-identification
state machine — a `Copy` enum threaded through detection, config, and pane
lifecycle, not merely storage. That surface did not shrink from the original
draft's estimate; only the bundled-storage sliver of it already moved. The
community-PR path captures the same user-facing value (a documented,
reviewable path to "shepherd recognizes my agent") at a fraction of the risk,
using infrastructure (regex caps, `regex::Regex`'s ReDoS-free guarantee,
`detection-golden-corpus` fixtures) that already exists and is already
enforced on every manifest the binary parses, remote or bundled.

- **Cheap intermediate (recommended):** a documented community-manifest
  submission path — a `website/agent-detection/` PR template plus the
  golden-screen fixtures from the `detection-golden-corpus` proposal as the
  acceptance criterion, plus an explicit maintainer regex-review step (the
  trust gap the automated caps don't cover). Still requires a release to
  activate (the new agent needs an `Agent` variant + bundled manifest before
  the catalog and the enum resolution machinery both above will accept it),
  but captures most of the contribution value with **no** architectural risk.
- **Full change:** a string-id detection path alongside the `Agent` enum so a
  remotely-delivered manifest can introduce a previously-unknown agent. Per the
  Coupling Inventory above, this touches `src/detect/mod.rs` (enum + both
  conversion functions), the manifest-cache/resolution chain in
  `src/detect/manifest.rs` and `manifest_update.rs`, `src/config/sound.rs`,
  `src/config/sidebar.rs`, `src/cli/spec.rs`, and — the part that did not
  shrink — `src/pane.rs`'s process-identification and lifecycle state machine.
  The wire schema (`src/api/schema/agents.rs`) and the hook-event path
  (`src/events.rs` `Hook*` variants) do NOT need to change; they are already
  string-native. Still a real trust question on top of the blast radius: a
  string id lets a community manifest introduce a *new* agent identity with no
  enum-derived allowlist backstop at all, so the regex-content-review gap
  above becomes load-bearing rather than a secondary concern.

## Acceptance (spike)

- A design note recommending one path with the blast-radius / trust analysis.
  (This document, plus `tasks.md`, is that note.)
- Community-PR path (chosen): the PR template + acceptance checklist exist and
  reference the golden-fixture test.
- `just check` passes if code lands (docs-only changes for this path).

## Explicitly not in scope

- De-enuming `Agent` in one change. "Curated agent list is the product" is a
  legitimate answer; the spike's job is the grounded decision, not the build.
- Actually building the string-id path. If a future proposal wants to revisit
  that decision, it starts from the Coupling Inventory above, not from
  scratch.
