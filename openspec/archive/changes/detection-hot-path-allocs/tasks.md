# Tasks — detection-hot-path-allocs

Base commit: `1de05dc2`. Drift check: confirm `src/detect/manifest.rs:414` still
defines `evaluate_loaded_manifest(..., include_update_status: bool)`, that `:338`
calls it with `false` and `:356` with `true`, and that `compiled_rule_matches`
(`:1180`) still does `text.to_lowercase()`. If changed, STOP and report.

Exemplar: the existing `include_update_status` parameter threading in
`src/detect/manifest.rs:338` vs `:356` — the flag that already distinguishes the
hot path from the explain path.

## Ordered steps

1. **Gate diagnostics behind the explain flag.**
   - In `evaluate_loaded_manifest`, only push to `evaluated_rules` and call
     `rule_evidence` when `include_update_status` is true. When false, leave
     `evaluated_rules` empty (the hot caller's `.into_detection()` ignores it —
     confirm `into_detection` does not read `evaluated_rules`).
   - Gate: `just test-one manifest` green; `just test-one detect` green.

2. **Hoist the lowercase copy out of the per-rule loop.**
   - Compute `lower_text` once per region (memoize per distinct `rule.region`
     within a single evaluation) and pass it into `compiled_gate_matches`. Only
     compute it when at least one gate needs case-insensitive matching — if
     determining that is non-trivial, computing once per region unconditionally
     is still a strict win over once per rule.
   - Gate: `just test-one manifest` green (matching behavior unchanged).

3. **Confirm the explain path is unaffected.**
   - Verify a test exercises `evaluate_loaded_manifest(..., true)` and asserts
     evidence is present; if not, add one asserting `evaluated_rules` is
     non-empty with populated `evidence` for a matching rule.
   - Gate: `just test-one explain` (or the relevant filter) green.

4. **Verify and close.**
   - `just check` passes. done-when: proposal `detection-hot-path-allocs`
     archived.

## STOP conditions

- If `into_detection()` (or any other non-explain caller) actually reads
  `evaluated_rules`, STOP — the diagnostics are not purely explain-only and the
  gating premise is wrong.
