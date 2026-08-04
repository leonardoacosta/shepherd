## Context

Agent sidebar rows accept eight built-ins, arbitrary `$custom` tokens, per-token foreground/bold/dim styles, row gaps, and per-agent overrides. Space rows use a different five-token vocabulary. Topbar rows reuse Agent tokens but render at a different width. Config limits allow 16 rows and 16 tokens per row, and the earlier configurable-chrome design intentionally kept arbitrary authoring in TOML.

The code establishes what can be rendered but not which small set of arrangements is useful. Product implementation therefore depends on evidence at actual narrow/normal widths and on a replacement contract that cannot silently flatten advanced configuration.

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

## Open Questions

- Which exact candidate names and contents survive the rendered comparison?
- Does every surface merit presets, or should one or more remain TOML-only?

Both questions are intentionally answered by the terminal evidence-dependent user gate, not during authoring.
