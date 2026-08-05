## Why

Base commit: **`81fd2237`** (branch `dev`).

`shepherd plugin outdated` is a shipped, user-invocable CLI subcommand with no user-facing
documentation in any locale.

It is declared in the CLI spec as `Command::new("outdated")` (`src/cli/spec.rs:826`) and dispatched
at `src/cli/plugin.rs:32` (`"outdated" => plugin_outdated(&args[1..])`). The implementation is
complete and tested: `PluginOutdatedStatus` with five states — `current`, `outdated`, `pinned`,
`n/a`, `unknown` (`src/cli/plugin.rs:213-225`, labels at `:388-395`), `PluginOutdatedReport`
(`:226`), human rendering via `print_plugin_outdated_human` (`:402`), a non-zero exit path when any
plugin is outdated (`:428`), and coverage at `:2138`.

It appears in none of the six maintained documentation surfaces. Grepping case-insensitively for
both `plugin outdated` and `plugin-outdated` returns zero hits in each of:

- `docs/next/website/src/content/docs/cli-reference.mdx`
- `docs/next/website/src/content/docs/plugins.mdx`
- `docs/next/website/src/content/docs/ja/cli-reference.mdx`
- `docs/next/website/src/content/docs/ja/plugins.mdx`
- `docs/next/website/src/content/docs/zh-cn/cli-reference.mdx`
- `docs/next/website/src/content/docs/zh-cn/plugins.mdx`

There is also no `docs/next/CHANGELOG.md` entry. The two `outdated` matches in that file
(lines 462 and 561) are older entries about integration status and sidebar update badges, not this
command.

This gap is already known and recorded. `docs/next/plugin-lifecycle-spike.md:113-117` lists it
verbatim under `## Follow-ups not in this spike`:

> - User docs for `plugin outdated` in `docs/next/website/src/content/docs/`
>   (`cli-reference.mdx`, `plugins.mdx`, plus the `ja/` and `zh-cn/` copies) and a
>   `docs/next/CHANGELOG.md` entry.

That spike's OpenSpec change was archived on 2026-08-04
(`openspec/changes/archive/2026-08-04-spike-plugin-lifecycle/`), so the code landed and the
follow-up did not travel with it. `docs/next/` is the staging area that becomes the next release's
published docs, which means the command currently ships to users who have no way to discover it
short of reading `--help` output or the Rust source.

The existing parity tooling does not catch this. `just release-docs-check` enforces that every
`*.mdx` has its `ja/` and `zh-cn/` counterparts (`justfile:100-116`) and runs
`scripts/docs_translation_parity.py` over both doc roots (`justfile:118-119`) — but all six files
are equally silent here, so parity holds and the gap passes. Nothing validates command coverage,
only locale symmetry.

## What Changes

- Document `shepherd plugin outdated` in `docs/next/website/src/content/docs/cli-reference.mdx`,
  following the structure the file already uses for its sibling `plugin` subcommands: synopsis,
  flags, and an example invocation with representative output.
- Add a short section to `docs/next/website/src/content/docs/plugins.mdx` covering the five status
  values and their meaning (`current`, `outdated`, `pinned`, `n/a`, `unknown`), and the non-zero
  exit when any plugin reports `outdated` — that exit behaviour is the scriptable contract and is
  the part a user cannot infer from the status names alone.
- Mirror both additions into `ja/` and `zh-cn/` copies of each file, per the locale parity gate at
  `justfile:100-119`.
- Add a `docs/next/CHANGELOG.md` entry under the unreleased section.
- Remove the now-satisfied follow-up bullet from `docs/next/plugin-lifecycle-spike.md:115-117`.

No Rust changes. `plugin outdated` already behaves correctly; this change documents it.

## Acceptance

- All six documentation files describe `plugin outdated`, including its five status values and its
  non-zero exit on an outdated plugin.
- `docs/next/CHANGELOG.md` carries an unreleased entry for the command.
- `docs/next/plugin-lifecycle-spike.md` no longer lists the documentation gap as an open follow-up.
- `just release-docs-check` passes, including its locale parity and `docs_translation_parity.py`
  steps.
