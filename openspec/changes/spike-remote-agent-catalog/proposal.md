# Design spike: let the remote catalog add agents, not just update the compiled-in ones

Base commit: `1de05dc2` · Route: proposal (design spike) · Effort: L (full) / M (community-PR path) · Confidence: MED (constraint is definite; loosening it is a maintainer call) · Category: direction

## Why

Resolves advisory finding DIRECTION-04 (audit against `1de05dc2`).

Recognizing a new coding agent is data (a manifest), but it is gated on a
compile-time enum and a release:

- `src/detect/manifest.rs:239-258` — `BUNDLED_MANIFESTS` is a
  `&[(&str, &str)]` of `include_str!` entries keyed to the `Agent` enum
  (`src/detect/mod.rs:43`).
- `src/detect/manifest_update.rs:379` — `remote_manifest_path(agent: Agent)` is
  typed on that same enum, so the remote catalog
  (`https://herdr.dev/agent-detection/index.toml`) can only ever **replace** a
  manifest for an agent the binary already knows, never add a new one.
- Adding a 20th agent requires touching `src/detect/mod.rs` (enum + `identify_agent`
  + `parse_agent_label` aliases), `src/detect/manifests/`, and
  `src/config/sound.rs` (per-agent sound settings), then waiting for a release.
  `CONTRIBUTING.md:41` routes such a feature through a Discussion + maintainer
  approval first.

New agents ship constantly, and "herdr recognizes my agent" is the single
highest-value outside contribution — it is data, not code. The out-of-band
delivery infra already exists and is tested (`src/detect/manifest/tests.rs` covers
remote/local/bundled precedence and version shadowing); only the enum key stands
in the way.

## What this spike produces

A decision between two grounded options, and the design for the chosen one:

- **Cheap intermediate (recommended first):** a documented community-manifest
  submission path — a `website/agent-detection/` PR template plus the
  golden-screen fixtures from the `detection-golden-corpus` proposal as the
  acceptance criterion. Still requires a release to activate, but captures most of
  the contribution value with **no** architectural risk.
- **Full change:** a string-id detection path alongside the `Agent` enum so a
  remotely-delivered manifest can introduce a previously-unknown agent. High blast
  radius — `Agent` is a `Copy` enum threaded through detection, config sounds, the
  wire schema (`src/api/schema/agents.rs`), and session snapshots — and a trust
  question, since a remote manifest is a regex-engine input (which is why
  `agent_detection_manifest_check.py` already caps rules/gate depth).

## Acceptance (spike)

- A design note recommending one path with the blast-radius / trust analysis.
- If the community-PR path is chosen: the PR template + acceptance checklist
  exist and reference the golden-fixture test.
- If the string-id path is chosen: a design doc enumerating every `Agent`-typed
  site that must accept a string id, plus the trust controls for remotely-added
  manifests — enough to scope a build proposal. Do not attempt the full de-enuming
  inside the spike.
- `just check` passes if code lands.

## Explicitly not in scope

- De-enuming `Agent` in one change. "Curated agent list is the product" is a
  legitimate answer; the spike's job is the grounded decision, not the build.
