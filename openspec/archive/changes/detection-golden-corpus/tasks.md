# Tasks — detection-golden-corpus

Base commit: `1de05dc2`. Drift check: confirm `src/detect/manifests/` still holds
~19 manifests, `scripts/capture_agent_screen.py` still writes to
`.local/agent-screen-captures`, and `src/detect/manifest/tests.rs` uses synthetic
manifests. If a golden corpus already exists, STOP and report.

Exemplar: existing fixture corpora under `tests/fixtures/` (keyboard/terminal
variants) for directory + loader style; `src/detect/manifest/tests.rs` for how to
drive the real detection engine against an input.

## Ordered steps

1. **Capture.** Using the throwaway-repro flow from AGENTS.md (§ Agent Detection
   Updates) and `scripts/capture_agent_screen.py`, drive each of the four thinnest
   agents (antigravity, github-copilot, kiro, amp) into idle/working/blocked and
   capture the bottom-buffer text (`herdr agent read <pane> --source detection
   --format text`).

2. **Scrub + commit fixtures.** Strip real paths, hostnames, usernames, and any
   token-like strings. Commit under `tests/fixtures/agent-screens/<agent>/<state>-N.txt`
   (≥3 per agent). These are data files, not gitignored.

3. **Write the table-driven test.** In `src/detect/` (or `tests/`), load each
   fixture, run the *bundled* manifest's detection for that agent, assert the
   result equals the label encoded in the path.
   - Gate: `just test-one detect` (or the new filter) green.

4. **Verify and close.**
   - `just check` passes. done-when: proposal `detection-golden-corpus` archived.

## STOP conditions

- If a captured fixture does NOT classify to its intended label with the current
  bundled manifest, STOP and report it as a detection finding — do not "fix" it by
  relabeling the fixture or editing the manifest inside this change.
- If capturing requires network/credentials for an agent you cannot run, skip
  that agent, note it, and proceed with the ones you can.
