## Context

Agent sidebar rows accept eight built-ins, arbitrary `$custom` tokens, per-token foreground/bold/dim styles, row gaps, and per-agent overrides. Space rows use a different five-token vocabulary. Topbar rows reuse Agent tokens but render at a different width. Config limits allow 16 rows and 16 tokens per row, and the earlier configurable-chrome design intentionally kept arbitrary authoring in TOML.

The code establishes what can be rendered but not which small set of arrangements is useful. Product implementation therefore depends on evidence at actual narrow/normal widths and on a replacement contract that cannot silently flatten advanced configuration.

**Drift reconfirmed at `shepherd@6aad4885` (task 1.1):** the token
vocabularies, style fields, limits, and gap/override fields above still match
`src/config/sidebar.rs` and `src/config/topbar.rs` exactly — no schema drift
since this proposal was authored. Two waves landed since the dependency
(`surface-settings-and-integration-controls`) applied: `expose-agent-state-source`
(typed `state_source` on pane/agent APIs) and `open-advanced-config-editor`
(`server.config.edit` editor pane). Neither touches the sidebar/topbar config
or token-resolution modules. `src/ui/settings.rs` already declares
`SettingsSection::Display` and `SettingsSection::Behavior` as candidate
landing sections, but renders nothing sidebar/topbar-related yet
(`rg -n "sidebar|topbar" src/ui/settings.rs` — no matches); a future picker
has a natural home but no existing UI to migrate. The advanced config editor
pane strengthens the "TOML remains sufficient" fallback option, since
operators can now hand-edit these exact row arrays in-app without a picker.
One real (non-breaking) drift note: `TopbarConfig` has no `row_gap` field, so
the nonzero-gap custom-classification rule in this design applies only to
Agent and Space sidebars, never to topbar rows. Full inventory, rendered
matrices, and the decision table live in
`evidence/layouts.md`.

## Goals / Non-Goals

**Goals:**

- Compare three to five concrete candidates for each applicable presentation surface.
- Capture production-equivalent rendering at all required widths and representative missing values.
- Finish a conservative custom classifier and exact preview/replacement contract.
- Produce an explicit go/no-go decision for a bounded follow-on feature.

**Non-Goals:**

- Implementing Settings controls, preview UI, config writes, or token reordering.
- Designing arbitrary token/style/custom-value editing.
- Clearing per-agent overrides implicitly or treating matching visible output as lossless round-tripping.

## Decisions

### Evidence uses serialized rows plus rendered cells

For each candidate, `evidence/layouts.md` records the exact TOML-equivalent row arrays and a monospaced cell rendering at sidebar widths 18, 24, and 36 or topbar widths 40 and 80. Representative values include long workspace/agent names, status tokens, absent metadata, and at least one value that elides. Focused `presentation_layout_candidate` tests in the existing token/topbar modules invoke production token resolution and truncation helpers; a hand-drawn mock is insufficient. These test-only additions remain after research as drift characterization but introduce no runtime branch or configuration behavior.

Rejected: evaluating preset names and raw arrays without seeing actual cells.

### Candidate families remain separate

Topbar, Agent rows, and Space rows receive separate candidate sets. Candidate count is three to five per surface, including the effective default/baseline where applicable. The evidence can reject a candidate or an entire surface if width behavior is consistently poor.

Rejected: force-fitting one row vocabulary across all surfaces.

### Custom detection is intentionally conservative

A configuration is custom when any token has explicit styling, any token is `$custom`, any applicable gap is nonzero, or any Agent `rows_by_agent` override exists. Exact equality with a known preset is recognized only after normalizing schema defaults; visual similarity is not sufficient. Adding a new style/token/override field must default to custom until classified.

Rejected: dropping unrecognized fields or declaring a layout preset-compatible because its current render happens to match.

### Preview precedes replacement

Any future picker must show the exact rows that will be written and a production-equivalent preview before persistence. Replacing a custom base layout requires explicit confirmation. Agent `rows_by_agent` remains untouched by default; clearing it requires a separate named opt-in in the same confirmation. Cancel produces no write.

The exact preset names and contents are deliberately reserved for the terminal evidence-dependent user gate after the comparison exists.

### Candidate recommendation (task 3.2, bounded to the rendered candidates in `evidence/layouts.md`)

This recommendation is bounded strictly to the candidates rendered in
`evidence/layouts.md` §2–§3; it does not introduce any candidate that was not
measured there, and it is not the terminal approval — task 5.1 owns that.

- **Agent sidebar**: a picker is evidence-supported. `baseline-compact`
  (current default) and `single-line` both stay legible from 18 columns up
  and degrade by dropping the tab token, not by mangling text — recommend
  carrying both forward to the gate. `status-first` and `full-context` are
  also evidence-supported but denser; recommend carrying them forward as
  secondary options rather than defaults. `terminal-title` is **not**
  recommended as rendered — §3.1/§6 show it renders only a state dot with no
  workspace fallback when both `terminal_title_stripped` and `agent` are
  absent, which is a real information loss the other four candidates do not
  have. It would need a `workspace` fallback token added and re-rendered
  before it is gate-eligible.
- **Space sidebar**: a picker is evidence-supported for all four rendered
  candidates (`baseline`, `single-line`, `status-first`, `branch-only`); none
  showed a missing-value or truncation defect at any measured width.
  `branch-only` trades away ahead/behind visibility, which the gate should
  weigh against `baseline`'s occasional empty second row (§3.2 note) rather
  than this research picking a winner.
- **Topbar**: recommend the gate treat this surface more cautiously than the
  two sidebars. `full-context` at 40 columns (§3.3) truncates all four of its
  tokens to single-digit remaining width — legible but dense enough that it
  may need a minimum-width guard the other candidates don't. If the gate
  wants a single safe default, `baseline-workspace-only` (current default) or
  `workspace-agent-two-row` are the least width-sensitive of the five
  rendered candidates. This is not a "no picker" recommendation — all five
  render correctly at both required widths — but it is a narrower one than
  the sidebars.
- No surface is recommended as TOML-only-with-no-picker based on this
  evidence; every rendered candidate produced legible output at every
  required width and value case except the one `terminal-title` gap named
  above.

## Risks / Trade-offs

- **Synthetic values make weak presets appear acceptable** → Include long, missing, and representative real-world-shaped values across every width.
- **Text evidence drifts from production rendering** → Generate or verify it through production token resolution and focused tests, recording the exact command and source revision.
- **Conservative custom detection reduces automatic preset recognition** → Prefer a false custom label to destructive rewriting; future schema-aware recognition can expand safely.
- **Research artifacts could be mistaken for implementation** → Keep source edits strictly test-only and require a separate feature after the post gate.

## Migration Plan

1. Apply the Settings/control prerequisite and refresh token/config semantics.
2. Add focused test-only characterization in existing topbar/sidebar modules without production behavior changes.
3. Record candidate arrays, rendered matrices, custom-classification cases, and comparison notes.
4. Complete the terminal user gate and record approved contents or the no-feature decision in this design/evidence set.
5. Archive the research change. Any implementation starts in a new OpenSpec change.

## Terminal gate disposition (task 5.1)

`decided-by: leo (delegated to the applying agent at apply time)`. The gate is a human
approval point; Leo delegated the call rather than reviewing the matrices himself, so this
disposition is recorded as delegated and remains open to reversal. Nothing here ships
behavior — it selects which candidates a future implementation change may author.

**Approved: 11 of 14 rendered candidates.**

- Agent sidebar: `baseline-compact`, `single-line`, `status-first`, `full-context`.
- Space sidebar: `baseline`, `single-line`, `status-first`, `branch-only`.
- Topbar: `baseline-workspace-only`, `workspace-tab`, `workspace-agent-two-row`, `status-first`.

**Rejected as-is: 2**, both on rendered output at a required width rather than on taste.

- **Agent `terminal-title`.** Alone among the fourteen it has no fallback identity: with
  neither `terminal_title_stripped` nor `agent` resolving it renders the state dot and
  nothing else (`evidence/layouts.md` §3.1 `terminal-title/missing`), where every other
  candidate degrades to the workspace name. A card that can render as a bare dot is not
  offerable. Re-propose only with a `workspace` fallback token appended, and re-measure.
- **Topbar `full-context`.** At 40 columns all four tokens truncate at once
  (§3.3 `full-context/long`). The Agent-sidebar `full-context` survives its 18-column floor
  because a usable workspace name remains; the topbar variant at its floor leaves four
  mutilated tokens. `workspace-agent-two-row` covers the dense case better by spending a
  second row instead of compressing one. Not re-proposable as a single row.

**Also unresolved, carried forward:** topbar `terminal_title` was rejected pre-render (§2.3)
on rationale rather than measurement. That rejection is accepted here on the same rationale
— the topbar duplicates the host terminal's own title bar — but it remains the one candidate
whose rejection is not backed by rendered evidence, and a future change may request it be
measured rather than treating this as settled.

Every surface keeps a picker; no surface is TOML-only. Each retains its current default as a
named candidate so a user can always return to it.

## Open Questions

None. Both prior questions are answered by the disposition above: eleven named candidates
survive the rendered comparison, and all three surfaces merit a picker.

Both questions are intentionally answered by the terminal evidence-dependent user gate, not during authoring.
