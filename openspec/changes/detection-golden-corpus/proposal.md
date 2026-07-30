# Build a golden-screen regression corpus for the detection manifests

Base commit: `1de05dc2` · Route: proposal · Effort: M · Confidence: HIGH · Category: test coverage

## Why

Resolves advisory finding TESTS-04 (audit against `1de05dc2`).

Agent-state classification (`blocked`/`working`/`idle`/`done`) is the feature
herdr exists for — it drives the sidebar, `agent wait`, `agent prompt --wait`,
and the socket API's status-changed event. Yet no bundled manifest's rules are
exercised against real captured agent output:

- `src/detect/manifests/` holds 19 agent manifests.
- `scripts/agent_detection_manifest_check.py:16-50` validates only *structure*
  (allowed keys, region grammar, version/rule/gate limits) — it never classifies
  a screen.
- `src/detect/manifest/tests.rs` builds *synthetic* manifests to test the engine;
  no real bundled manifest is checked against captured output.
- `scripts/capture_agent_screen.py:19` writes captures to
  `.local/agent-screen-captures`, which `.gitignore:14` excludes — so the
  evidence AGENTS.md requires when editing a manifest is discarded after the edit.
- Reference density is lopsided: `pi` 278 mentions across src+tests, `claude`
  119, `codex` 107, versus `antigravity` 2, `github-copilot` 2, `kiro` 4.

A regex tightened for one agent can silently regress another, and nothing in CI
notices.

## What changes

Commit a small per-agent corpus of captured bottom-buffer snapshots (idle /
working / blocked) under `tests/fixtures/agent-screens/<agent>/<state>-N.txt`,
plus one table-driven test asserting each **bundled** manifest classifies its
labeled fixtures to the expected state. Paths/hostnames scrubbed on commit. Start
with the four thinnest manifests (antigravity, github-copilot, kiro, amp).

This also pays for DIRECTION-04 (community agent submissions): the fixtures become
the acceptance criterion for a new-agent manifest PR.

## Acceptance

- `just check` passes.
- `tests/fixtures/agent-screens/<agent>/` exists for at least the four thinnest
  manifests with ≥3 labeled snapshots each.
- A table-driven test loads each fixture, runs the bundled manifest's detection,
  and asserts the classified state equals the label. Gate: `just test-one detect`
  (or the new test's filter) green.
- Captures contain no real paths, hostnames, or tokens (scrub verified).

## Out of scope

- Changing any manifest's rules — this is coverage only. If a fixture reveals a
  misclassification, capture it as a separate finding rather than tuning the
  manifest inside this change.
- Full 19-agent coverage in one pass — start with the thin four; the test should
  make adding more purely additive.
