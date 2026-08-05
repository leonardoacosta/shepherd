## 1. Confirm the gap before editing

- [ ] 1.1 Confirm the base commit still matches. Run `git rev-parse --short HEAD` and expect
  `81fd2237`, or confirm the cited sites still hold: `Command::new("outdated")` at
  `src/cli/spec.rs:826` and `"outdated" => plugin_outdated(&args[1..])` at `src/cli/plugin.rs:32`.
  If the subcommand has been renamed or removed, **STOP and report the drift**.
- [ ] 1.2 Re-confirm zero documentation coverage. Run
  `grep -ric 'plugin outdated\|plugin-outdated' docs/next/website/src/content/docs/cli-reference.mdx docs/next/website/src/content/docs/plugins.mdx docs/next/website/src/content/docs/{ja,zh-cn}/{cli-reference,plugins}.mdx`
  and expect `0` for all six files. If any is non-zero, narrow this change to the remaining files.
- [ ] 1.3 Capture the real output to document from, rather than inventing it. Run
  `cargo run -- plugin outdated --help` and `cargo run -- plugin outdated`, clearing inherited
  socket overrides per `AGENTS.md`:
  `env -u SHEPHERD_SOCKET_PATH -u SHEPHERD_CLIENT_SOCKET_PATH cargo run -- plugin outdated`.
  Use the actual rendering from `print_plugin_outdated_human` (`src/cli/plugin.rs:402`) as the
  example block.

## 2. Document the command in English

- [ ] 2.1 Add a `plugin outdated` entry to `docs/next/website/src/content/docs/cli-reference.mdx`,
  matching the structure the file already uses for its sibling `plugin` subcommands — synopsis,
  flags, example invocation, example output from 1.3. Place it in the existing `plugin` subcommand
  ordering, not appended at the end of the file.
- [ ] 2.2 Add a status-values section to `docs/next/website/src/content/docs/plugins.mdx` covering
  all five states from `plugin_outdated_status_label` (`src/cli/plugin.rs:388-395`): `current`,
  `outdated`, `pinned`, `n/a`, `unknown`. State what each means for the user, not just the label.
- [ ] 2.3 Document the exit-code contract in the same section: the command exits non-zero when any
  plugin reports `outdated` (`src/cli/plugin.rs:428`). This is the scriptable behaviour and cannot
  be inferred from the status names, so it needs its own sentence.

## 3. Mirror into maintained locales

- [ ] 3.1 Add the corresponding `plugin outdated` sections to
  `docs/next/website/src/content/docs/ja/cli-reference.mdx` and `ja/plugins.mdx`.
- [ ] 3.2 Add the corresponding sections to
  `docs/next/website/src/content/docs/zh-cn/cli-reference.mdx` and `zh-cn/plugins.mdx`.
- [ ] 3.3 Run `python3 scripts/docs_translation_parity.py --docs-root docs/next/website/src/content/docs`
  and expect exit 0. Then run `python3 -m unittest scripts.test_docs_translation_parity` and expect
  all tests to pass.

## 4. Close the loop

- [ ] 4.1 Add an unreleased entry for `plugin outdated` to `docs/next/CHANGELOG.md`. Keep the
  subject descriptive — per `AGENTS.md`, commit subjects and changelog lines feed preview release
  notes. Do not touch root `CHANGELOG.md`; `justfile:90` requires the two to match only at release
  time, and `just release` performs that copy.
- [ ] 4.2 Remove the satisfied follow-up bullet at `docs/next/plugin-lifecycle-spike.md:115-117`.
  If it is the only remaining item under `## Follow-ups not in this spike`, remove the heading too
  rather than leaving an empty section.
- [ ] 4.3 Re-run the docs sweep and confirm no regression:
  `docs-sweep --json | jq '.summary'` — `flagged` must not increase relative to the pre-change
  value of `2`, and `error` must stay `0`.
- [ ] 4.4 Run `just release-docs-check` and expect exit 0, then `just check` and expect all
  repository checks to pass. Propose the commit message and get alignment before committing, per
  `AGENTS.md`.
