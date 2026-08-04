# Plugin Lifecycle Spike: `plugin outdated` and the Auto-Update Question

Date: 2026-07-30
Status: accepted — manual drift reporting only; no automatic updates
Scope: design spike for a plugin upgrade / drift path (openspec `spike-plugin-lifecycle`)

## What shipped in this spike

`shepherd plugin outdated [--plugin ID] [--json]` — a read-only, user-invoked drift
report. It never fetches plugin code, never checks anything out, and never runs a
plugin command. Per installed plugin it runs exactly one
`git ls-remote -- <remote> <ref>` and compares the returned SHA against the
`source.resolved_commit` already recorded at install time.

Provenance itself needed no new code: `PluginSourceInfo` (`src/api/schema/plugins.rs`)
has recorded `requested_ref`, `resolved_commit`, `managed_path`, and
`installed_unix_ms` since the install path landed, and `InstalledPluginInfo.version`
records the declared manifest version. The one gap was that nothing proved a
pre-`source` registry still deserializes; that is now a regression test in
`src/persist/plugin_registry.rs`.

### What it compares

| Recorded state | Reported status | How |
| --- | --- | --- |
| `kind = local` (includes `plugin link`, and any pre-`source` registry entry) | `n/a` | No upstream exists. Not an error. |
| `kind = github`, `requested_ref` is a branch/tag (or absent) | `current` / `outdated` | `ls-remote` the ref (`HEAD` when absent, matching how install resolves a missing `--ref`), compare to `resolved_commit`. |
| `kind = github`, `requested_ref` is an exact commit SHA | `pinned` | An exact commit has no "newer" answer. No network call is made. |
| `kind = github`, no `resolved_commit` recorded | `unknown` | Legacy entry; nothing to diff. No network call is made. |
| `kind = github`, ref missing / ambiguous upstream / `ls-remote` failed | `unknown` + labelled detail | Best-effort per plugin; one bad plugin never fails the sweep. |

Annotated tags return both the tag object and its peeled `^{}` commit; the peeled
entry wins, because install resolves to the commit.

### Deliberate non-choices

- **No marketplace-index comparison.** `workers/plugin-marketplace/src/index.ts`
  (`PluginListing`) is a GitHub topic-search discovery cache: stars, forks,
  `pushedAt`, no version, no commit, keyed by repo and not by subdir. It
  structurally cannot answer "is my `resolved_commit` stale", and using
  `pushedAt` alone would false-positive every monorepo-subdir plugin whose repo
  was pushed to for unrelated reasons. Live `ls-remote` is the only honest source.
- **No poller, no cache file.** The command is manual. `src/detect/manifest_update.rs`
  is the in-repo exemplar for cached scheduled remote checks, but adopting its
  scheduling is precisely the decision below, not a free mechanism to inherit.
- **No API method, no server state.** The report is a client-side comparison of
  data the registry already holds, so the report type lives in `src/cli/plugin.rs`.
  If drift ever needs to surface in the TUI (a badge, a settings row), that is the
  moment it becomes a shared runtime fact and earns a neutral server/API surface —
  it should not be back-doored through the private TUI client socket.
- **Exit code is 0 even when plugins are outdated** (usage error `2`, unknown
  `--plugin` `1`). Drift is a report, not a failure. If CI-style gating is wanted
  later, add an explicit `--exit-code` flag rather than changing the default.

## The decision: conservative vs. opt-in auto-update

### Option A — conservative: `outdated` only (recommended)

Shepherd reports drift; the user re-runs `shepherd plugin install <owner>/<repo>[/subdir]`
when they want it. Install already prints a full preview (actions, startup
commands, event hooks, panes, build commands) and requires confirmation, so every
new line of third-party code that gains execution rights passes a human eye.

### Option B — expansive: opt-in auto-update

A user opts a plugin (or all plugins) into automatic upgrade; Shepherd periodically
checks and pulls new upstream commits.

### The tradeoff that decides it

There is **no reviewed, versioned plugin release channel to trust**. The
marketplace index is an unreviewed GitHub topic-search cache — inclusion means a
repo carries a topic, not that anyone vetted it — and it carries no version or
commit anyway. Auto-update therefore means *auto-pulling arbitrary new upstream
commits with zero review gate*, on plugins whose manifests declare `startup`
commands, event hooks, and build commands that Shepherd executes. A plugin author
losing their GitHub account, or shipping one bad commit, converts silently into
code execution on every machine that installed it. Today the blast radius of a
compromised upstream is bounded by the user choosing to reinstall; auto-update
removes that bound.

The `min_shepherd_version` check (`src/app/api/plugins/manifest.rs`) does not help
here — it gates compatibility, not trust.

### Recommendation

**Ship Option A. Do not build auto-update yet.** One-line reason: with no vetted,
versioned release channel, auto-update converts "user chose to run this code" into
"any future upstream commit runs on this machine", and the marketplace index
cannot supply the missing review gate.

The owner accepted Option A. Concretely:

1. Keep `plugin outdated` manual and read-only, exactly as shipped.
2. Reconsider auto-update only once at least one of these exists: signed or
   maintainer-reviewed plugin releases; a version-and-commit-keyed marketplace
   entry (`PluginListing` gaining `version` + `commit`, keyed by subdir); or a
   capability/permission model that makes a plugin's execution rights explicit
   and re-confirmable on change.
3. The plausible middle step, if drift reporting proves useful, is a **passive
   notice** — a cached background `outdated` check that only *tells* the user,
   reusing `src/detect/manifest_update.rs`'s cache/status-file mechanics with no
   fetch of plugin code. That is a separate proposal: it introduces
   unattended network calls to third-party remotes and a per-user opt-out, and it
   is where the TUI surface (and therefore a server/API-owned drift fact) would be
   designed properly.

Any TUI surface or unattended background drift check remains undecided and must
be proposed separately with its server/API ownership, network privacy, and trust
model. The manual command's `git` dependency is accepted; plugin installation
already requires it, so this adds no new runtime dependency for plugin users.

## Follow-ups not in this spike

- User docs for `plugin outdated` in `docs/next/website/src/content/docs/`
  (`cli-reference.mdx`, `plugins.mdx`, plus the `ja/` and `zh-cn/` copies) and a
  `docs/next/CHANGELOG.md` entry.
