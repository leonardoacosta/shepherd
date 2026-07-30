# Stop building detection diagnostics on the hot detection path

Base commit: `1de05dc2` · Route: proposal · Effort: S · Confidence: HIGH · Category: performance

## Why

Resolves advisory finding PERF-06 (audit against `1de05dc2`).

Agent-state detection runs every ~300ms per agent pane. On each tick the manifest
evaluator builds a full diagnostics record for **every** rule and then, on the
hot path, throws it all away:

- `src/detect/manifest.rs:423-446` (`evaluate_loaded_manifest`) — for every rule
  it pushes an `EvaluatedRule` with `rule.id.clone()`, `rule.region.clone()`, and
  `rule_evidence(rule, region_text)`:
  ```rust
  evaluated_rules.push(EvaluatedRule {
      id: rule.id.clone(),
      priority: rule.priority,
      region: rule.region.clone(),
      evidence: rule_evidence(rule, region_text),
      ...
  });
  ```
- `src/detect/manifest.rs:1183-1203` (`rule_evidence`) clones `rule.contains`,
  `rule.regex`, `rule.line_regex` (`Vec<String>` each) and builds a 240-char
  `bounded_preview` (`text.chars().take(240).collect()` plus a full
  `text.chars().count()`).
- `src/detect/manifest.rs:1180` (`compiled_rule_matches`) allocates
  `text.to_lowercase()` — a full copy of the screen region — **per rule**, even
  when the rule's gate has no case-insensitive `contains` needles.
- The hot caller discards all of it: `src/detect/manifest.rs:338`
  `evaluate_loaded_manifest(agent, input, loaded, false).into_detection()`. Only
  `herdr agent explain` (`:356`, called with `true`) consumes `evaluated_rules`.

A Claude pane (12 rules) pays 12× `to_lowercase()` of the screen region, 12×
`Vec<String>` clone sets, and 12× preview construction every 300ms, all to build
a struct that is immediately dropped. With 10 agent panes that is a continuous
burst of wasted allocation.

## What changes

- Only build `evaluated_rules` / `rule_evidence` when `include_update_status`
  (the explain flag) is `true`. The parameter already exists and already
  threads through — this is gating existing work behind it, not new plumbing.
- Compute the lowercased region text once per region (not once per rule), and
  only when some gate actually needs it.

Detection results (`into_detection()`) must be byte-for-byte identical; only the
diagnostics-building work is skipped on the hot path. `herdr agent explain`
output must not change.

## Acceptance

- `just check` passes.
- Detection unit tests unchanged and green (`just test-one detect`,
  `just test-one manifest`).
- A test (or manual `herdr agent explain <pane> --json` capture) confirms the
  explain path still emits the full `evaluated_rules` with evidence.

## Out of scope

- The matching semantics in `compiled_gate_matches` — do not change what matches,
  only how many times the lowercase copy is made.
- The `EvaluatedRule` / `RuleEvidence` struct shapes consumed by `agent explain`.
