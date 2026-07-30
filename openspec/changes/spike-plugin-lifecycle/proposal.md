# Design spike: a plugin upgrade / outdated path

Base commit: `1de05dc2` · Route: proposal (design spike) · Effort: M · Confidence: MED (the gap is certain; owning plugin lifecycle is a product call) · Category: direction

## Why

Resolves advisory finding DIRECTION-03 (audit against `1de05dc2`).

Every installed plugin is frozen at install time:

- `src/cli/spec.rs:765-774` — `plugin install` accepts `--ref` to pin a git ref;
  there is no `plugin upgrade`, `plugin update`, or `plugin outdated` subcommand
  and no corresponding API method.
- `src/app/api/plugins/manifest.rs` enforces `min_herdr_version` in one direction
  only (rejects a plugin needing a newer herdr); nothing notices a plugin being
  stale relative to upstream.
- `workers/plugin-marketplace/wrangler.toml` — the marketplace re-indexes every
  30 minutes, so the index knows about new plugin versions installed plugins can
  never reach.
- `src/persist/plugin_registry.rs` records `installed_unix_ms` but no upstream ref
  or version to diff against.

A plugin author shipping a fix has no channel to reach users; a user must
`uninstall` + `install` by hand. The detection-manifest subsystem already solved
exactly this: `src/detect/manifest_update.rs` pulls a remote index on a schedule
with version comparison and cache fallback — an in-repo, tested template.

## What this spike produces

A decision + minimal implementation for the **conservative** increment first:

- **`plugin outdated`** (read-only): record the resolved upstream ref/version at
  install in `plugin_registry`, and add a command/API that reports which installed
  plugins have newer upstream versions. No auto-fetch, no execution — user decides.

And a written position on the expansive option (opt-in auto-update), which imports
the marketplace's *unreviewed-index* security posture into the update path and
must not be conflated with the conservative one.

## Acceptance (spike)

- `src/persist/plugin_registry.rs` records the resolved upstream ref/version at
  install time (schema addition, migration-safe).
- A `plugin outdated` command (and/or API method) reports drift for installed
  plugins against the marketplace index / recorded ref.
- A design note records the auto-update decision (do it opt-in, or "plugins are
  pinned, period" — a defensible answer worth writing down).
- `just check` passes.

## Explicitly not in scope

- Auto-updating third-party plugin code. That is a separate proposal with its own
  security review; this spike delivers read-only drift reporting plus the decision.
