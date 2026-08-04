## Why

Shepherd currently gives two kinds of guidance that can be false: onboarding hardcodes shortcuts even when effective bindings differ, and unreleased Hermes documentation claims lifecycle authority that the runtime deliberately does not grant. Align both surfaces with their authoritative runtime sources so first-run and integration guidance remains trustworthy.

## What Changes

- Render onboarding's prefix, help, and Settings shortcuts from the effective keybinding configuration through the existing shared formatting vocabulary.
- Wrap long or multiple shortcut values into a readable multi-line block at narrow terminal widths instead of truncating the instruction.
- Correct the English, Japanese, and Chinese unreleased Hermes documentation to describe session identity plus screen-manifest state detection and to use the bundled integration version.
- Keep keybinding semantics, onboarding flow, Hermes runtime behavior, and stable released documentation unchanged.

## Capabilities

### New Capabilities

- `truthful-user-guidance`: Runtime-derived onboarding shortcuts and unreleased Hermes guidance that agrees with the shipped integration contract.

### Modified Capabilities

None.

## Impact

- UI: onboarding rendering and its focused tests.
- Documentation: next-version agent and integration guides in all maintained locales; stable website pages remain untouched.
- Runtime/API: no behavior, schema, protocol, persistence, or integration-asset change.
- Dependencies: no new dependency.
- Issue linkage: not applicable because this repository has no `.beads` store and the user requested a fork-local feature queue from recorded research.
- Base and baseline:
  - base-commit: shepherd@062955ae513d4b0e3281e957043b190bdfd90ee6
  - dirty-baseline: untracked `improvements.md` is user-requested advisory research and is excluded from this feature.
- touches: `src/ui/onboarding.rs`, `src/ui/keybind_help.rs`, `docs/next/website/src/content/docs/integrations.mdx`, `docs/next/website/src/content/docs/agents.mdx`, `docs/next/website/src/content/docs/ja/integrations.mdx`, `docs/next/website/src/content/docs/ja/agents.mdx`, `docs/next/website/src/content/docs/zh-cn/integrations.mdx`, `docs/next/website/src/content/docs/zh-cn/agents.mdx`, `scripts/test_docs_translation_parity.py`

## Preconditions

- Reconfirm that onboarding still hardcodes the default prefix/help values and that the menu/help surfaces still expose the effective formatter before implementation.
- Reconfirm `HERMES_INTEGRATION_VERSION` and the session-only authority tests at the implementation base; update the proposal if the runtime contract has changed.
- Preserve unrelated worktree changes and edit only unreleased documentation.

## Decisions

- Use the same effective binding values and human-readable formatter as menus/help. This is `decided-by: user` through the selected recommendation. Rejected: duplicating key syntax or retaining default-only copy.
- Show all configured alternatives needed to invoke prefix, help, and Settings, and wrap the shortcut block at narrow widths. This is `decided-by: default` because omission would still make onboarding misleading. Rejected: clipping or choosing an arbitrary first binding.
- Describe Hermes as session-identity reporting with state supplied by screen detection, and derive the documented asset version from the current constant during verification. This is `decided-by: user` through the selected correction. Rejected: changing runtime authority to match stale prose.
- Keep stable documentation unchanged until its corresponding release contract is independently confirmed. Rejected: broad documentation edits that could describe unreleased behavior as stable.

## Done Means

- Default and custom prefix/help/Settings bindings render truthfully, including a readable 40-column case with no lost shortcut.
- English, Japanese, and Chinese next docs consistently describe Hermes session identity, screen-derived state, and the current integration version.
- Focused onboarding tests, translation parity, documentation checks, `git diff --check`, and `just check` pass with no runtime, protocol, stable-doc, or integration-asset diff.

## Testing

- Run `just test-one onboarding`; expected result: default, customized, alternative-binding, and 40-column rendering tests pass.
- Run `python3 -m unittest scripts.test_docs_translation_parity scripts.test_hermes_integration_asset`; expected result: locale parity and Hermes asset/version assertions pass.
- Run `git diff --check && git diff --exit-code -- src/detect src/integration website/src/content/docs`; expected result: no whitespace error and no Hermes runtime, integration asset, or stable documentation change.
- Run `openspec validate align-onboarding-and-hermes-guidance --strict --no-interactive`; expected result: strict validation exits 0.
- Run `just check`; expected result: all repository checks pass.
