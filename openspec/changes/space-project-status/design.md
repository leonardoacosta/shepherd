# Design — space-project-status

## Shape

This change is `git_refresh.rs` a second time, with a different probe and a slower clock. The
existing module is the template, not just an inspiration: every structural decision below already
has a working precedent 200 lines away, and deviating from it would be the thing that needs
justifying.

```
src/app/git_refresh.rs                 src/app/project_status_refresh.rs   (new)
  git_refresh_demand()          ──►      project_status_demand()
    reads sidebar_spaces.rows              reads sidebar_spaces.rows
    → GitStatusRefreshDemand               → ProjectStatusRefreshDemand
      { branch, ahead_behind }               { proposals, beads }

  workspace_git_refresh_items() ──►      project_status_refresh_items()
  deduplicate_..._items()       ──►      deduplicate_..._items()
    key: repo_root                         key: checkout_key
  start_..._if_due(now)         ──►      start_..._if_due(now)
  ..._deadline()                ──►      ..._deadline()
```

## Why `checkout_key`, not `repo_root`

Git status is a property of the repository, so `git_status_cache_key_for_space` canonicalizes
`space.repo_root` and two worktrees of one repo legitimately share an entry. Project status is a
property of the *working tree*: `openspec/changes/` is tracked content that differs per branch,
and `.beads/` is commonly untracked and per-checkout. `GitSpaceMetadata` already distinguishes
these — `key` groups a space, `checkout_key` identifies the checkout — so the correct key exists
and needs no new discovery work.

## Why two tokens rather than one

cc-tmux's row-3 rule is the borrowed precedent: two independent segments, each omitted entirely
when its own counts are unavailable, so a broken half never blanks a valid half. Shepherd's token
layer already gives this for free — per-token elision is existing behaviour — but only if the two
counts are two tokens. A single combined `project_status` token would make the beads database's
absence blank the proposals count, which is the exact failure cc-tmux wrote a rule to prevent.

Shepherd deliberately does **not** reproduce cc-tmux's `ua` closure-debt count or its
standalone-versus-tracked bead partitioning. Both are derivations over cc's own conventions
(`[SPEC]`/`[CAPABILITY]` title prefixes, archive discipline) that `openspec list --json` does not
expose and Shepherd has no business inferring.

## Provider contract

Two hardcoded providers, invoked with the checkout as the working directory:

| Token | Command | Parsed to |
| --- | --- | --- |
| `proposals` | `openspec list --json` | `changes[]` → open count, in-progress count |
| `beads` | `bd list --status open --json` | array → open, ready, blocked counts |

Rendered forms, matching the compactness of the existing `git_status` token:

```
op: 2o 1ip          bd: 3o 1r 0b
```

Every failure mode collapses to `None` for that token alone: binary absent, non-zero exit,
timeout, non-JSON body, JSON of an unexpected shape. `None` is already the elision signal in
`SpaceTokenContext`, so no new render path is needed.

## Cadence

Git status polls frequently because `git status` is cheap and branch changes are the thing an
operator watches second-to-second. Two subprocess spawns per checkout at that cadence is a
different cost class, and proposal/issue counts move on the order of minutes. The refresh
therefore carries its own interval and its own deadline rather than riding the git tick, and the
deadline is reported alongside the git deadline at the three existing wiring sites
(`src/app/runtime.rs`, `src/server/headless.rs`, `src/server/headless/scheduled_tasks.rs`).

## Rejected: configurable provider commands

A `[project_status.providers]` config accepting arbitrary commands would generalise this to any
tool. It is rejected for now: there is no third tool to generalise from, it adds config surface
for a requirement nobody has stated, and it turns a config file into a scheduler for arbitrary
subprocesses. Git is hardcoded; these two are hardcoded for the same reason. If a third provider
ever appears, that is the moment to extract the shape — not before.
