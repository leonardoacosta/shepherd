## Why

Shepherd's configurable topbar and sidebars can represent styled tokens, custom values, row gaps, and per-agent overrides, so a simplistic visual composer could silently destroy valid configurations. Before proposing implementation, Shepherd needs rendered evidence for useful bounded presets and an explicit custom-preservation contract.

## What Changes

- Produce three to five candidate layouts separately for topbar, Agent rows, and Space rows.
- Render every candidate at 18, 24, and 36 sidebar columns and 40 and 80 topbar columns using representative values and missing-value cases.
- Add focused test-only characterization that exercises candidate output through production token resolution and rendering rules.
- Classify styled tokens, `$custom` tokens, nonzero gaps, and per-agent overrides conservatively as custom.
- Decide preview, explicit replacement, and `rows_by_agent` preservation semantics from the evidence, or record that TOML should remain the only editor.
- Do not implement a picker, composer, config writer, Settings row, or new runtime behavior in this change.

## Capabilities

### New Capabilities

- `presentation-layout-preset-decision`: Durable rendered evidence and an approved bounded contract for any future presentation-layout picker.

### Modified Capabilities

None.

## Impact

- Research and test surfaces: the OpenSpec design/evidence and focused test-only characterization in existing topbar/sidebar token modules are updated during apply.
- Product/runtime: test-only source characterization is allowed, but no runtime branch, config, API, protocol, documentation, or persisted-state behavior changes.
- Follow-on: an implementation feature may be authored only after the terminal evidence gate approves exact presets and replacement semantics.
- Feature dependency: `surface-settings-and-integration-controls` MUST be applied and archived before this research begins so candidate placement reflects the final responsive Settings information architecture.
- depends on: `surface-settings-and-integration-controls`
- Issue linkage: not applicable because this repository has no `.beads` store and the user requested a fork-local feature queue from recorded research.
- Base and baseline:
  - base-commit: shepherd@062955ae513d4b0e3281e957043b190bdfd90ee6
  - dirty-baseline: untracked `improvements.md` is user-requested advisory research and is excluded from this feature.
- touches: `openspec/changes/research-presentation-layout-presets/design.md`, `openspec/changes/research-presentation-layout-presets/evidence/layouts.md (new)`, `src/ui/sidebar/tokens.rs`, `src/ui/topbar.rs`

## Preconditions

- Apply `surface-settings-and-integration-controls` first so the research evaluates the resulting Display/Behavior information architecture rather than competing with it.
- Reconfirm the current Agent, Space, and topbar token vocabularies, style fields, row/token limits, gap behavior, and per-agent override semantics before generating candidates.
- Use representative actual values and the real rendering/token-elision rules; do not judge names or TOML shapes alone.

## Decisions

- Keep topbar, Agent rows, and Space rows as separate preset families because their vocabularies and geometry differ. This is `decided-by: user` through the research decision map. Rejected: one shared preset shape.
- Treat any style, `$custom` value, nonzero gap, or per-agent override as custom. This is `decided-by: user`. Rejected: best-effort matching that could flatten advanced configuration.
- A future preset choice must show the exact resulting rows before replacement and require explicit confirmation for custom layouts. Agent `rows_by_agent` remains untouched unless the confirmation separately names and opts into clearing it. This is `decided-by: user` as the minimum safety contract; exact candidate contents remain a later-evidence-dependent human action.
- The terminal post gate may approve a bounded candidate set, request one evidence revision, or conclude that TOML remains sufficient. Rejected: authoring the implementation feature before that gate.

## Done Means

- One durable evidence document and focused characterization tests record three to five candidates per applicable surface, every required width, representative/missing values, truncation/elision behavior, and custom-detection outcomes.
- The design records the exact preview and replacement contract, selected preset contents and names, treatment of `rows_by_agent`, and rejected candidates—or records why no picker should be built.
- A terminal evidence-dependent user gate records the disposition without changing product code.
- Strict OpenSpec validation and `git diff --check` pass, and a follow-on implementation feature is either decision-complete or explicitly not warranted.

## Testing

- Run the repository's focused sidebar and topbar render tests named during apply and capture the exact command in `evidence/layouts.md`; expected result: candidate output uses production token resolution, style, elision, and truncation rules.
- Run `rg -n '\$custom|rows_by_agent|gap|fg|bold|dim' src/config/sidebar.rs src/config/topbar.rs`; expected result: the evidence checklist covers every custom-classifying field found in the current schema.
- Run `openspec validate research-presentation-layout-presets --strict --no-interactive && git diff --check`; expected result: both commands exit 0 and the diff remains confined to this change's research artifacts and declared test-only characterization paths.
