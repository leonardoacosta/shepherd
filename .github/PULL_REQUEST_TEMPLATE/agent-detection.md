<!--
Agent detection manifest submission. Read website/agent-detection/SUBMITTING.md
first — it defines every field, cap, and command referenced below.

This template is for: adding/revising a bundled screen-detection manifest
under src/detect/manifests/, its fixtures under tests/fixtures/agent-screens/,
and (for a genuinely new agent) the identity wiring in src/detect/mod.rs.
It is not for lifecycle-hook integrations, general bug fixes, or features —
use the default PR flow for those.
-->

## What this changes

- [ ] Revises rules for an agent Shepherd already bundles
- [ ] Proposes a new agent identity (links the identity-wiring change in
      `src/detect/mod.rs` — `Agent`, `agent_label`, `parse_agent_label`,
      `Agent::SCREEN_MANIFEST_AGENTS` — plus `BUNDLED_MANIFESTS` in
      `src/detect/manifest.rs`, since a manifest alone can't introduce an
      unknown identity)

Agent: <!-- id, e.g. codex -->

## Evidence (SUBMITTING.md § 1)

- [ ] Each proposed rule is backed by `shepherd agent read <pane> --source
      detection --format text` output captured from the real agent CLI, not
      from memory or another agent's screen
- [ ] ANSI evidence (`--format ansi`) is included where styling or
      alternate-screen behavior affects the rule
- [ ] Every alternative valid control for the same state is captured and
      represented as an explicit `any`/`all` OR-path in the manifest, not a
      single incidental string match

<!-- Paste the captured detection-source text (or a representative excerpt)
     for each state/control variant this PR adds or changes. -->

## Manifest and fixtures (SUBMITTING.md § 2-4)

- [ ] Manifest fields match the documented shape; no fields beyond `id`,
      `version`, `min_engine_version`, `updated_at`, `aliases`, `rules`
- [ ] `min_engine_version` is the lowest version that supports every
      region/feature used, not just the current engine version
- [ ] Fixtures added under `tests/fixtures/agent-screens/<agent>/` are the
      smallest representative set (one per state, one per distinct control
      variant) — not a large per-agent full-screen suite

## Automated checks (SUBMITTING.md § 5)

Paste the actual output of each command run against this branch:

```
$ python3 scripts/agent_detection_manifest_check.py
<paste output>
```

```
$ cargo nextest run --locked "manifest::tests"
<paste output>
```

```
$ just check
<paste output>
```

## Matcher content review (SUBMITTING.md § 6 — maintainer, not automated)

Passing the checks above proves structure and bounds only. Reviewer must
separately confirm:

- [ ] No matcher is likely to fire on unrelated/incidental screen text
- [ ] Each matcher's intent matches an invariant control the agent actually
      shows for that state
- [ ] `state`/`visible_idle`/`visible_blocker`/`visible_working` labels match
      what a user would see
- [ ] Every fixture added maps to a rule this PR actually exercises

## Delivery

Identity stays curated and bundled: this manifest becomes active detection
only in the next Shepherd release that ships it, per `website/agent-detection/SUBMITTING.md`.
