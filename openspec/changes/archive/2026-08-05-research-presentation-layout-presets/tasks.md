## 1. Refresh the presentation contract

- [x] 1.1 Confirm `surface-settings-and-integration-controls` is applied/archived, inventory current topbar/Agent/Space vocabularies, style fields, limits, gaps, overrides, token elision, and renderer entry points, and update `design.md` for real drift. Run `rg -n '\$custom|rows_by_agent|gap|fg|bold|dim|MAX_.*ROWS|MAX_.*TOKENS' src/config/sidebar.rs src/config/topbar.rs`; expected result: every custom-classifying field and bound is represented in the research checklist.
  - touches: `openspec/changes/research-presentation-layout-presets/design.md`
  - depends on: surface-settings-and-integration-controls

## 2. Produce rendered candidate evidence

- [x] 2.1 Define three to five exact TOML-equivalent candidates separately for Agent rows, Space rows, and topbar rows, including the current effective baseline where applicable; record rationale and rejected candidates. Expected result: `evidence/layouts.md` contains named exact arrays with no shared-vocabulary assumption.
  - touches: `openspec/changes/research-presentation-layout-presets/evidence/layouts.md`
  - depends on: 1.1

- [x] 2.2 Add focused `presentation_layout_candidate` characterization that uses production token resolution to render Agent and Space candidates at 18, 24, and 36 columns with long, representative, and missing values. Run `just test-one presentation_layout_candidate`; expected result: every sidebar candidate/width cell matrix passes against production elision, styling, and truncation behavior.
  - touches: `src/ui/sidebar/tokens.rs`, `openspec/changes/research-presentation-layout-presets/evidence/layouts.md`
  - depends on: 2.1

- [x] 2.3 Add matching topbar `presentation_layout_candidate` characterization at 40 and 80 columns through production resolution/rendering. Run `just test-one presentation_layout_candidate`; expected result: every topbar candidate/width matrix and missing-value case passes.
  - touches: `src/ui/topbar.rs`, `openspec/changes/research-presentation-layout-presets/evidence/layouts.md`
  - depends on: 2.1

## 3. Finish safe recognition and replacement semantics

- [x] 3.1 Build a decision table for exact preset matches, styled tokens, `$custom`, nonzero gaps, per-agent overrides, unknown future fields, cancel, base-row replacement, and separately confirmed override clearing. Expected result: every specification scenario has one explicit outcome in `evidence/layouts.md` with conservative custom fallback.
  - touches: `openspec/changes/research-presentation-layout-presets/evidence/layouts.md`
  - depends on: 2.2, 2.3

- [x] 3.2 Compare readability, information density, missing-value behavior, and replacement risk; update `design.md` with the candidate recommendation while leaving exact approval to the terminal evidence gate. Expected result: the recommendation is bounded to exact candidates or explicitly recommends no picker for a surface.
  - touches: `openspec/changes/research-presentation-layout-presets/design.md`
  - depends on: 3.1

## 4. Verify the research packet

- [x] 4.1 Run `just test-one presentation_layout_candidate` and `rg -n '\$custom|rows_by_agent|gap|fg|bold|dim' src/config/sidebar.rs src/config/topbar.rs`; expected result: generated evidence matches current production behavior and every current custom field maps to conservative classification.
  - touches: none (read-only validation)
  - depends on: 3.2

- [x] 4.2 Run `openspec validate research-presentation-layout-presets --strict --no-interactive && git diff --check && git diff --exit-code -- Cargo.toml Cargo.lock src/protocol docs website`; expected result: strict/whitespace checks pass and the research apply diff contains no dependency, runtime protocol, or product documentation change.
  - touches: none (read-only validation)
  - depends on: 4.1

## User Gate

- [x] 5.1 [user:post] Review the rendered matrices and safe-replacement table, then approve exact preset names/contents, request one evidence revision, or choose TOML-only; apply records the disposition in `design.md`/`evidence/layouts.md`, reruns `just test-one presentation_layout_candidate && openspec validate research-presentation-layout-presets --strict --no-interactive && git diff --check` with the expected result that characterization and final artifacts pass, and only then marks this terminal task complete. No implementation feature is authored without approval.
  - touches: `openspec/changes/research-presentation-layout-presets/design.md`, `openspec/changes/research-presentation-layout-presets/evidence/layouts.md`
  - depends on: 4.2
