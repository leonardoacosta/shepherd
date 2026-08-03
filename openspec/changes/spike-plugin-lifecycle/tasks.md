# Tasks — spike-plugin-lifecycle (design spike)

Base commit: `c000681f`. Drift check: re-derive these before touching anything, then
compare against what you find:

- `PluginSourceInfo` (`src/api/schema/plugins.rs:71-88`) already records `kind`,
  `owner`, `repo`, `subdir`, `requested_ref`, `resolved_commit`, `managed_path`,
  `installed_unix_ms`; `InstalledPluginInfo.version` (`plugins.rs:40`) already
  records the declared manifest version. Both are written at install time by
  `to_source_info` / `load_cli_plugin_manifest` (`src/cli/plugin.rs:204`).
  Provenance recording is DONE — do not re-implement it.
- `src/cli/spec.rs:762-776` (`plugin_command`) still has no `outdated`/`upgrade`/
  `update` subcommand. This drift claim is still true.
- `workers/plugin-marketplace/src/index.ts:53-82` (`PluginSnapshot`/`PluginListing`)
  is a GitHub topic-search discovery cache (stars/forks/pushedAt per repo) — it has
  NO version or commit field. It cannot serve as a version-comparison source for
  `plugin outdated`; only a live upstream git ref can.

If any of the three bullets above no longer hold, STOP — the spike needs a fresh
re-scope, not another patch to this file.

Exemplar: `src/detect/manifest_update.rs` is the proven in-repo pattern for the
*mechanics* (cached remote check, skip-if-unchanged, atomic write of the result) —
reuse its cache/status-file shape. Its *data source* does NOT transfer: it compares
`ManifestVersion` (dotted-numeric, parsed from a Shepherd-owned `index.toml` keyed by
agent) against a cached value, and the plugin marketplace index carries no
equivalent version field per plugin (see drift check above). `plugin outdated`
needs its own comparison source — see Step 2.

## Steps

1. **Verify provenance is already correct; close the one real gap (test
   coverage, not code).** Confirm `PluginSourceInfo` and `InstalledPluginInfo.version`
   already carry everything the original Step 1 asked for (see Drift check).
   Confirm migration-safety is already structurally true: `InstalledPluginInfo.source`
   is `#[serde(default)]` (`plugins.rs:62-63`) and `PluginSourceInfo` has a custom
   `Default` impl (`plugins.rs:90-103`) plus per-field `#[serde(default,
   skip_serializing_if = "Option::is_none")]`, so a pre-existing registry JSON
   written before `source` existed still deserializes via `load_from_path_strict`.
   The one residual gap: `src/persist/plugin_registry.rs` has round-trip /
   missing-file / corrupt-file / reload tests but none load a source-less JSON
   literal — add that one regression test to make the migration-safety claim
   checkable, not just structurally-plausible. Note explicitly and do not "fix":
   `plugin link` of a local directory (`src/cli/plugin.rs` `plugin_link`, always
   passes `source: None`) leaves `PluginSourceInfo::default()` (`kind: Local`, no
   `installed_unix_ms`) — correct behavior, not a gap, since a local plugin has no
   upstream to diff against and Step 2 only applies to `Github`-kind sources.
   - Gate: `just test-one plugin_registry` green, including the new source-less
     JSON regression test.

2. **Implement `plugin outdated` (read-only), comparing against the upstream git
   ref — not the marketplace index.** For each installed `Github`-kind plugin, run
   a live `git ls-remote <remote_url> <requested_ref-or-HEAD>` (no local checkout,
   no fetch of plugin code) and compare the returned SHA against the stored
   `source.resolved_commit`. Reuse `run_git`/`command_failure_message`
   (`src/cli/plugin.rs:860+`) with `cwd: None` for the bare `ls-remote` — no new
   git-invocation plumbing needed. `Local`-kind plugins have nothing to diff
   against; report them as `n/a`, never as an error. If `requested_ref` was
   already a specific commit SHA (not a branch/tag), there is no "newer" answer —
   report it as `pinned`, not `outdated`/`current`. The marketplace index may be
   consulted only as an optional secondary signal (`pushedAt` on the matching
   `owner/repo`, `index.ts:65-68`) if a fully offline heads-up is wanted later —
   but it cannot replace the `ls-remote` check: it isn't keyed by `subdir` and
   carries no commit/version to compare, so on its own it would false-positive
   every monorepo-subdir plugin whose repo was pushed to elsewhere.
   - Gate: `just test-one plugin` green; manual run against a linked GitHub-kind
     test plugin whose upstream has moved shows it reported outdated, and a
     plugin already at the remote tip reports current.

3. **Write the auto-update decision note** under `docs/next/` or `.local/prd/`:
   conservative (`outdated`-only via live `ls-remote`) vs. opt-in auto-update.
   Spell out that there is no reviewed, versioned plugin release channel to trust
   for auto-update — the marketplace index is an unreviewed GitHub discovery
   cache (Step 2's drift check), not a vetted registry, so auto-updating means
   auto-pulling arbitrary new upstream commits with zero review gate. Recommend
   one.

4. **Verify and close.** `just check` passes. done-when: proposal
   `spike-plugin-lifecycle` archived with the decision recorded, `plugin outdated`
   merged, and the Step 1 regression test present.

## STOP conditions

- If `git ls-remote` needs network access this spike deliberately wants to avoid
  running unattended (a manual, user-invoked `plugin outdated` is fine; wiring it
  into a background/scheduled auto-poll is the Step 3 decision's job, not this
  step's) — report and stop rather than silently adding a poller.
- If matching an installed plugin's `owner`/`repo`/`requested_ref` to a live
  upstream ref is ambiguous (e.g. `requested_ref` was empty and resolved via
  `HEAD` at install, or the named ref was a tag that has since moved) — report
  best-effort per plugin and label the ambiguity; do not fail the whole command
  over one plugin.
