# Design spike: a plugin upgrade / outdated path

Base commit: `c000681f` · Route: proposal (design spike) · Effort: M · Confidence: MED (the gap is certain; owning plugin lifecycle is a product call) · Category: direction

> Re-scoped 2026-07-30 (base commit bumped from `1de05dc2`): provenance recording
> landed ahead of this spec. `PluginSourceInfo` already records the resolved
> upstream ref/commit and `InstalledPluginInfo.version` already records the
> declared version, both migration-safe. The remaining work is `plugin outdated`
> itself plus the auto-update decision note — see tasks.md for the sharpened plan.

## Why

Resolves advisory finding DIRECTION-03 (audit against `1de05dc2`).

Every installed plugin is frozen at install time:

- `src/cli/spec.rs:762-776` — `plugin install` accepts `--ref` to pin a git ref;
  there is no `plugin upgrade`, `plugin update`, or `plugin outdated` subcommand
  and no corresponding API method. (Still true.)
- `src/app/api/plugins/manifest.rs:229` (`validate_min_herdr_version`) enforces
  `min_herdr_version` in one direction only (rejects a plugin needing a newer
  herdr); nothing notices a plugin being stale relative to upstream. (Still true.)
- `workers/plugin-marketplace/wrangler.toml:15` — the marketplace re-indexes
  every 30 minutes (`crons = ["*/30 * * * *"]`), but its snapshot
  (`workers/plugin-marketplace/src/index.ts:53-82`) is a GitHub topic-search
  discovery cache with no version or commit field per plugin — it cannot answer
  "is this installed plugin stale" on its own. (Corrected: originally assumed
  the index carried per-plugin versions; it does not.)
- `src/persist/plugin_registry.rs` / `src/api/schema/plugins.rs:71-88` — ~~records
  `installed_unix_ms` but no upstream ref or version to diff against~~. **This is
  now false.** `PluginSourceInfo` already records `requested_ref`,
  `resolved_commit`, and `installed_unix_ms`; `InstalledPluginInfo.version`
  already records the declared manifest version. Both are written at install
  time (`src/cli/plugin.rs:204`) and are migration-safe via serde defaults.

A plugin author shipping a fix has no channel to reach users; a user must
`uninstall` + `install` by hand. The detection-manifest subsystem solved the
*mechanics* of this problem (`src/detect/manifest_update.rs`: scheduled remote
check, cache, atomic write) but not the *data source* — its remote index is
Herdr-owned and version-keyed, unlike the plugin marketplace's discovery cache,
so only the mechanics transfer; `plugin outdated` needs a live upstream git ref
check as its actual comparison source (see tasks.md Step 2).

## What this spike produces

A decision + minimal implementation for the **conservative** increment first:

- **`plugin outdated`** (read-only): compare each installed `Github`-kind
  plugin's recorded `resolved_commit` against a live `git ls-remote` of its
  upstream ref, and report drift. No auto-fetch, no execution — user decides.

And a written position on the expansive option (opt-in auto-update), which imports
the marketplace's *unreviewed-index* security posture into the update path and
must not be conflated with the conservative one.

## Acceptance (spike)

- A `plugin outdated` command (and/or API method) reports drift for installed
  plugins against a live upstream git ref, using the provenance already recorded
  in `plugin_registry`.
- A regression test proves a pre-existing (source-less) registry JSON still
  loads under the current schema (migration-safety, currently unexercised by
  any test).
- A design note records the auto-update decision (do it opt-in, or "plugins are
  pinned, period" — a defensible answer worth writing down).
- `just check` passes.

## Explicitly not in scope

- Auto-updating third-party plugin code. That is a separate proposal with its own
  security review; this spike delivers read-only drift reporting plus the decision.
