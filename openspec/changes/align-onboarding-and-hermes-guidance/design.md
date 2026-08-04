## Context

`src/ui/onboarding.rs` currently owns a literal `ctrl+b` constant and renders `?` as a combined keybind/help/settings instruction. In contrast, `AppState` already carries the effective prefix key plus `ActionKeybinds`, and `src/ui/keybind_help.rs` formats configured actions through `ActionKeybinds::label()` while prefix UI uses `format_key_combo`. The onboarding popup is capped at 64 columns and its shortcut instruction currently receives a single row, so replacing short literals with arbitrary configured labels also requires deterministic wrapping.

Hermes' bundled Python plugin reports only a resumable session reference. Terminal authority deliberately excludes Hermes from full-lifecycle reporting, leaving state classification to `src/detect/manifests/hermes.toml`. The next-version English and translated docs instead describe lifecycle authority and include a stale integration version. Stable docs are a separate released contract and are not part of this correction.

## Goals / Non-Goals

**Goals:**

- Source onboarding shortcut copy from the same effective configuration displayed elsewhere.
- Keep every required shortcut visible at 40 columns and preserve render purity.
- Make every maintained next-doc locale agree with Hermes' session-only runtime and current asset version.

**Non-Goals:**

- Changing keybinding parsing, defaults, prefix semantics, onboarding progression, or modal input.
- Changing the Hermes plugin, manifest, authority arbitration, or integration version.
- Editing stable docs or adding a new localization system.

## Decisions

### Centralize action labels, not onboarding prose

Expose the existing `ActionKeybinds::label()` fallback as a small shared UI helper next to the current keybinding-help formatting and continue to call `format_key_combo` for prefix. Onboarding composes those values into its own prose. The helper returns `unset` when an action has no binding, matching keybind help and avoiding a false executable instruction.

Rejected: calling `keybind_help_groups` and searching its presentation rows. That would couple onboarding to help grouping and labels rather than the authoritative binding values.

### Give shortcuts a measured multi-line block

Build semantic lines for prefix, help, and Settings, then wrap them within the popup content width. The shortcut block gets enough rows to display all lines at the 40-column contract; descriptive prose yields space before shortcuts do. Tests inspect the buffer for each complete formatted binding rather than hard-coded coordinates alone.

Rejected: truncate-to-fit or render only a first alternative. Both can remove the only usable configured binding.

### Correct next docs against constants and tests

State that Hermes reports session identity and that the screen manifest remains responsible for state. Use the current bundled integration version in all locales, and add a narrow docs parity assertion that checks the same version/authority concepts without forcing word-for-word translations.

Rejected: editing the runtime to agree with the old docs or copying changes into stable documentation without release verification.

## Risks / Trade-offs

- **Long customized labels can consume onboarding space** → Use a bounded multi-line shortcut block and exercise 40-column rendering with multiple bindings.
- **A formatter extraction can subtly change help text** → Characterize current help labels before sharing the helper and assert existing default output.
- **Translations can drift semantically while satisfying file parity** → Check Hermes version plus session/state authority markers in each maintained locale and retain human-readable translated prose.

## Migration Plan

1. Add characterization tests for default/custom onboarding and the current keybind formatter.
2. Share the formatter and render the measured shortcut block.
3. Correct next-version Hermes docs and strengthen narrow parity checks.
4. Run focused and repository-wide verification. Rollback is a source/docs revert; no stored state or wire data migrates.

## Open Questions

None.
