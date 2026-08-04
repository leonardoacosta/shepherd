# Submitting an agent detection manifest

This guide covers community contributions to Shepherd's screen-manifest agent
detection: adding a manifest for a new agent, or tightening the rules for an
agent Shepherd already knows. It does not cover lifecycle-hook integrations
(`src/integration/`) or process identification outside screen detection.

Read `CLAUDE.md` § "Agent Detection Updates" and § "Universal Project Rules"
first. Two rules from there are non-negotiable for every submission:

- **Detection is decoupled.** The detector reads a screen snapshot; it never
  touches the parser or viewport state.
- **Screen detection is evidence-based.** Match the invariant controls a
  state actually shows, not incidental whole-pane text, and never treat the
  user-visible scrollback as detection authority — it can be scrolled.

## What a submission can and cannot do

Shepherd's agent identity is curated and bundled, not remotely extensible:
`parse_catalog` (`src/detect/manifest_update.rs`) drops any remote catalog
entry whose id doesn't resolve through the binary's known agent labels, and
`scripts/agent_detection_manifest_check.py` enforces the same rule for
`website/agent-detection/index.toml`. That boundary does not change here.

- **Revising an already-bundled agent's rules** needs only a manifest change
  under `src/detect/manifests/<agent>.toml` plus fixtures.
- **Proposing a genuinely new agent** additionally needs the identity wired
  into `src/detect/mod.rs` (the `Agent` enum, `agent_label`,
  `parse_agent_label`, `Agent::SCREEN_MANIFEST_AGENTS`) and
  `src/detect/manifest.rs`'s `BUNDLED_MANIFESTS` table — a maintainer-owned
  code change, reviewed alongside the manifest and fixtures.

Either way, the agent becomes detectable only in the next Shepherd release
that ships the binary containing it. Merging the manifest does not turn on
detection by itself, and a remote catalog update can only ever revise an
agent the running binary already identifies.

## 1. Capture evidence from the detector's authoritative source

Drive the target agent's real CLI into each state you intend to match, then
read the exact text the detector evaluates:

```bash
shepherd agent read <pane> --source detection --format text
```

Add `--format ansi` when styling or alternate-screen behavior affects the
rule (e.g. a state only distinguishable by color, or a screen that only
appears in the terminal's alternate buffer).

Do this once per representative, distinct state you're proposing a rule for
— idle, working, blocked — and once per distinct invariant control when a
state has more than one valid visible form (e.g. a permission prompt that
sometimes reads "Allow once / Deny" and sometimes "y/n"). Never use
`--source visible`, `recent`, or `recent-unwrapped` as detection evidence —
those reflect what a user sees and can scroll away from what the engine is
evaluating.

Once you have a draft manifest, use `shepherd agent explain <pane> --json`
(or `--format text`) to confirm it classifies the pane the way you expect
before you cut fixtures from the capture.

## 2. Manifest shape

Top-level fields (`src/detect/manifest.rs`'s `AgentManifest`, unknown fields
rejected):

| field | type | notes |
| --- | --- | --- |
| `id` | string | canonical agent id, e.g. `"codex"` |
| `version` | string | dotted numeric, e.g. `"2026.07.24.1"` |
| `min_engine_version` | integer | see § Engine version below |
| `updated_at` | string | informational, RFC 3339 timestamp |
| `aliases` | array of strings | additional ids/labels this manifest matches |
| `rules` | array of rule tables | see below, non-empty, max 128 entries |

Each rule (`ManifestRule`) and each nested gate (`ManifestGate`) accepts:

| field | applies to | notes |
| --- | --- | --- |
| `id` | rule only | non-empty, unique within the manifest |
| `state` | rule only | `"idle"` \| `"working"` \| `"blocked"` \| `"unknown"` |
| `priority` | rule only | integer, higher wins when multiple rules match |
| `region` | rule only | see § Regions below; defaults to `whole_recent` |
| `visible_idle` / `visible_blocker` / `visible_working` | rule only | bool, diagnostic evidence flags |
| `skip_state_update` | rule only | bool; requires `state = "unknown"` and no `visible_*` flags |
| `all`, `any`, `not` | rule and gate | arrays of nested gates |
| `contains`, `regex`, `line_regex` | rule and gate | arrays of string matchers |

A gate (the rule's own top level, or any `all`/`any` entry) must contain at
least one positive matcher (`contains`, `regex`, `line_regex`, `any`, or
`all`); `not` entries must contain at least one matcher of any kind. Prefer
`any` to express alternative valid controls for the same state instead of
picking one incidental string.

### Regions

`whole_recent`, `whole_recent_without_current_prompt_marker`,
`after_last_prompt_marker`, `before_current_prompt_marker`,
`current_prompt_block_marker`, `after_current_prompt_block_marker`,
`prompt_box_body`, `above_prompt_box`, `last_non_empty_above_prompt_box`,
`after_last_horizontal_rule`, `osc_title`, `osc_progress`,
`bottom_lines(N)`, `bottom_non_empty_lines(N)`, `top_non_empty_lines(N)`.

`N` must be a positive integer, capped at 65535. `top_non_empty_lines(N)`
additionally requires `min_engine_version >= 3`.

### Engine version

`min_engine_version` is checked against `MANIFEST_ENGINE_VERSION` in
`src/detect/manifest_update.rs` (currently `3`). Set it to the lowest engine
version that can evaluate every region/feature your manifest uses — don't
bump it further than that just because it's the current release.

## 3. Structural caps

Enforced identically by the Rust engine (`src/detect/manifest.rs`) and
`scripts/agent_detection_manifest_check.py`:

| limit | value |
| --- | --- |
| rules per manifest | 128 |
| gate nesting depth | 8 |
| total gates per manifest | 512 |
| matchers per gate | 32 |
| total matchers per manifest | 1024 |
| characters per matcher string | 512 |
| `top_non_empty_lines`/`bottom_non_empty_lines`/`bottom_lines` line count | 65535, and `top_non_empty_lines` needs `min_engine_version >= 3` |

These caps bound resource use only. They do not prove a matcher's *meaning*
is correct — that's a human review step, not an automated one (§ 6).

## 4. Fixtures

Golden fixtures live at `tests/fixtures/agent-screens/<agent>/<state>-<n>.txt`,
where `<agent>` is any id or alias the binary already resolves through
`parse_agent_label` (`src/detect/mod.rs`) for a bundled agent, and `<state>`
is `idle`, `working`, or `blocked`. Content is the raw, unmodified output of
`shepherd agent read <pane> --source detection --format text` from step 1 —
capture verbatim, don't hand-edit it into something more "representative"
than what the agent actually shows.

The fixture directory alone is enough: `cargo nextest run --locked
"manifest::tests::bundled_manifest_golden_screen_fixtures_match_labeled_state"`
discovers every directory under `tests/fixtures/agent-screens/`, resolves it
to a bundled manifest, and asserts every fixture inside classifies as its
filename's state. Adding fixtures for an agent Shepherd already bundles
requires no Rust test-code changes.

Keep this to the **smallest representative set**: one fixture per distinct
state, plus one more per distinct invariant control variant within a state
(see § 1). Do not build a large per-agent full-screen fixture suite for
routine manifest tuning — `CLAUDE.md` § "Agent Detection Updates" bans that,
and the existing Rust suite stays focused on manifest parsing, rule
semantics, skip-state semantics, source precedence, cache reload behavior,
and update flow rather than exhaustive agent-specific screens.

## 5. Local validation

Before opening a PR:

```bash
python3 scripts/agent_detection_manifest_check.py
cargo nextest run --locked "manifest::tests"
just check
```

The Python check validates manifest shape and every structural cap in § 3
for every bundled manifest (and, when `website/agent-detection/` exists
locally, the published catalog against it). The Rust suite runs manifest
parsing/semantics tests plus the golden fixture test from § 4. `just check`
is the full repository gate; run it before opening the PR, not instead of
the two commands above.

## 6. Maintainer review

Passing §5 proves the manifest is well-formed and inside its structural
bounds — it says nothing about whether a matcher means what you think it
means. A maintainer separately reviews:

- false-positive risk: could this matcher fire on unrelated output?
- matcher intent: does each `contains`/`regex`/`line_regex` target an
  invariant control, not incidental text that happened to be on screen?
- state labels: does the assigned `state`/`visible_*` combination match what
  a user would actually see?
- fixture-to-classification coverage: does every fixture in § 4 map to a
  rule that's actually exercised, not just a directory that happens to pass?

Use the PR template at `.github/PULL_REQUEST_TEMPLATE/agent-detection.md`
(`https://github.com/leonardoacosta/shepherd/compare/dev...you:branch?template=agent-detection.md`
or the template picker when opening the PR) — it asks for the evidence,
check output, and fixture coverage this guide describes, plus the identity
wiring PR link if you're proposing a new agent.
