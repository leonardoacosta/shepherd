## Why

A space already knows what project it is. `Workspace` carries `identity_cwd`, a discovered
`GitSpaceMetadata { key, checkout_key, repo_name, repo_root, is_linked_worktree }`, a cached
branch, and cached ahead/behind counts, and `[ui.sidebar.spaces]` already renders `branch` and
`git_status` tokens from them. Git is the only project fact Shepherd surfaces.

For a coding-agent runtime, git is half the picture. The other half is the work queue: how many
change proposals are open, and how many tracked issues are open, ready, or blocked. An operator
watching four spaces cannot tell which project has work waiting without leaving Shepherd.

Two facts already in the tree make this cheap rather than new infrastructure:

- `src/app/git_refresh.rs` is a complete, demand-driven, deduplicated refresh loop that runs off
  the render path. `git_refresh_demand()` inspects `sidebar_spaces.rows` and refreshes **only**
  what the configured rows actually consume, so an operator who configures no git tokens pays
  nothing. `deduplicate_git_refresh_items` collapses several workspaces resolving to the same
  cache key into one probe.
- The space token layer already elides an absent value and drops a fully unresolved row, so a
  probe that fails needs no placeholder and no error surface.

Project status is a shared runtime fact about a checkout, not a TUI presentation detail, so the
server owns it — the same ownership git status already has.

## What Changes

- Add `proposals` and `git`-sibling `beads` tokens to the space token vocabulary, resolving to
  compact count strings (`op: 2o 1ip`, `bd: 3o 1r 0b`).
- Add a native, demand-driven project-status refresh that mirrors `git_refresh.rs`: it runs only
  when a configured space row consumes one of the new tokens, deduplicates by checkout, runs off
  the render path, and caches results with an independent, longer interval than git status.
- Shell out to `openspec list --json` and `bd list --json` from that refresh, per checkout.
- Fail open per segment. A missing binary, a non-zero exit, a timeout, or an unparseable body
  yields no value for that token only; the other token and every git token are unaffected.
- Document the two tokens on the unreleased docs path.

Nothing about pane, tab, or agent state changes. No existing token changes meaning. An operator
who does not configure the new tokens sees no behaviour change and pays no new process cost.

## Capabilities

### New Capabilities

- `space-project-status` — per-checkout project work-queue status, refreshed natively on demand
  and exposed through the space token vocabulary.

### Modified Capabilities

None. No committed spec covers the `[ui.sidebar.spaces]` token vocabulary today, so this change
adds a capability rather than extending one.

## Impact

- `src/config/sidebar.rs` — two new `SpaceSidebarToken` variants, their names, and parsing.
- `src/workspace.rs` — cached per-workspace project-status fields and their snapshot type.
- `src/app/project_status_refresh.rs` (new) — the refresh loop, demand resolution, and dedup.
- `src/app/mod.rs` — module declaration.
- `src/app/runtime.rs`, `src/server/headless.rs`, `src/server/headless/scheduled_tasks.rs` —
  deadline and due-check wiring beside the existing git-refresh calls.
- `src/ui/sidebar/tokens.rs` — token resolution into `SpaceTokenContext`.
- `docs/next/website/src/content/docs/` — token reference for the unreleased path.
- Shepherd gains an optional runtime dependency on `openspec` and `bd` being on `PATH`. Neither
  is required: absent binaries resolve to absent tokens, exactly as an absent `$custom` token
  does today.

## Preconditions

- premise: `openspec list --json` emits a stable, parseable shape with per-change task counts —
  verified: run against this repo, returns `{"changes":[{"name","completedTasks","totalTasks",
  "lastModified","status"}]}` @ shepherd@cfb8358d
- premise: `bd list --status open --json` emits a parseable array carrying `status` per issue —
  verified: run in `~/dev/claude`, exit 0, array of objects with `id`/`status`/`priority`
  @ shepherd@cfb8358d
- premise: `bd` is absent in this repo and exits non-zero, which is the fail-open path this
  change must handle — verified: `bd search` here returns `Error: no beads database found`
  @ shepherd@cfb8358d
- premise: git status refresh is already demand-driven off the configured space rows — verified:
  `git_refresh_demand()` iterates `self.state.sidebar_spaces.rows` and sets `branch`/
  `ahead_behind` only for the `Branch`/`GitStatus` tokens, `src/app/git_refresh.rs:102-112`
  @ shepherd@cfb8358d
- premise: refresh work is already deduplicated per cache key and run off the render path —
  verified: `deduplicate_git_refresh_items`, `src/app/git_refresh.rs:136`, and its test
  `git_refresh_deduplicates_workspaces_with_same_cache_key`, `src/app/git_refresh.rs:208`
  @ shepherd@cfb8358d
- premise: a per-checkout key already exists and is distinct from the space grouping key —
  verified: `GitSpaceMetadata { key, checkout_key, .. }`, `src/workspace/git/discovery.rs`
  @ shepherd@cfb8358d
- premise: the space token layer elides an absent value without a placeholder — verified:
  `SpaceTokenContext` resolution in `src/ui/sidebar/tokens.rs` and the existing
  omission-of-unresolved-row behaviour asserted in `src/ui/sidebar.rs` tests @ shepherd@cfb8358d

## Decisions

- Who computes the counts — chosen: Shepherd computes them natively on its own refresh timer,
  like git status; rejected: an external producer pushing values through
  `shepherd workspace report-metadata --token`, which works today and was verified live, because
  it requires every operator to install and schedule a separate poller; decided-by: leo
- Provider generality — chosen: two built-in providers, `openspec` and `bd`, hardcoded exactly as
  git is hardcoded; rejected: a configurable provider-command shape, because no third tool exists
  to generalise from, it adds config surface for a requirement nobody has, and it would let a
  config file schedule arbitrary commands on a timer; decided-by: default
- Segment independence — chosen: two separate tokens that elide independently; rejected: one
  combined token, because a missing `bd` database would then blank a valid proposals count;
  decided-by: default
- Cache key — chosen: key by `GitSpaceMetadata.checkout_key`; rejected: keying by `repo_root` as
  git status does, because linked worktrees hold separate `openspec/changes/` and `.beads/`
  contents and would report each other's counts; decided-by: default
- Refresh interval — chosen: an independent interval, longer than the git-status interval, with
  its own deadline; rejected: reusing the git-status tick, because two subprocess spawns per
  checkout at git cadence is a materially heavier idle cost than `git status`;
  decided-by: default
- Count vocabulary — chosen: proposals as open/in-progress and beads as open/ready/blocked,
  sourced directly from each tool's own JSON; rejected: reproducing cc-tmux's `ua` closure-debt
  and standalone-vs-tracked partitioning, because those are consumer-specific derivations that
  Shepherd cannot compute from `openspec list --json` alone; decided-by: default

## Done Means

- An operator can add `proposals` and `beads` tokens to a `[ui.sidebar.spaces]` row and see
  per-space counts render beside the existing branch and git-status tokens.
- A space whose checkout has no `openspec/changes/` and no beads database renders the row with
  those tokens elided and every other token intact — no placeholder, no error.
- A space whose checkout has proposals but no beads database renders the proposals count and
  elides only the beads token.
- An operator who configures neither token observes no new subprocess spawned by Shepherd.
- Several spaces pointing at the same checkout produce one probe per refresh, not one per space.

## Testing

- A demand test asserting the refresh does not start when no configured space row consumes
  either token, mirroring `due_git_refresh_does_not_start_without_sidebar_consumer`.
- A demand test asserting each token independently enables only its own provider.
- A dedup test asserting workspaces sharing a `checkout_key` collapse to one refresh item, and a
  counter-test asserting two linked worktrees of one repo do NOT collapse.
- Parser tests over captured `openspec list --json` and `bd list --json` bodies, plus
  malformed-body, empty-array, and non-zero-exit cases resolving to `None`.
- Render tests asserting an absent count elides the token and its separator, that one absent
  segment leaves the other rendering, and that a row of only absent tokens is omitted.
- Config tests asserting both token names parse, round-trip through serialization, and reject an
  unknown name.
- `just check` passes.
