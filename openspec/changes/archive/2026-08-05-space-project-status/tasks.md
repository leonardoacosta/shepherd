## 1. Extend the space token vocabulary

- [x] 1.1 In `src/config/sidebar.rs`, add `Proposals` and `Beads` variants to `SpaceSidebarToken`,
      and their names `proposals` and `beads` in `space_token_name`. Do not touch
      `AgentSidebarToken` — the two vocabularies stay separate.
- [x] 1.2 Add a config test asserting both names parse from a `[ui.sidebar.spaces]` row, survive a
      serialize/deserialize round trip, and carry a configured `fg`/`bold`/`dim` style through
      `parts()`.
- [x] 1.3 Add a config test asserting an unknown space token name is rejected with a
      configuration error.
- [x] 1.4 Run `cargo nextest run config::` and paste the passing output.

## 2. Cache project status on the workspace

- [x] 2.1 In `src/workspace.rs`, add a `ProjectStatusSnapshot` carrying optional proposal counts
      (open, in-progress) and optional bead counts (open, ready, blocked), with each half
      independently optional so one can be absent while the other resolves.
- [x] 2.2 Add the cached snapshot and its cache key to `Workspace`, alongside the existing
      `cached_git_*` fields, and an accessor mirroring `git_ahead_behind()`.
- [x] 2.3 Add a unit test asserting a workspace with no cached snapshot reports both halves
      absent.
- [x] 2.4 Run `cargo nextest run workspace::` and paste the passing output.

## 3. Add the demand-driven refresh

- [x] 3.1 Create `src/app/project_status_refresh.rs` and declare it in `src/app/mod.rs` beside
      `mod git_refresh`.
- [x] 3.2 Implement `project_status_demand()` reading `self.state.sidebar_spaces.rows` and setting
      one flag per token, exactly as `git_refresh_demand()` does at
      `src/app/git_refresh.rs:102`. Do not read any other config source.
- [x] 3.3 Implement refresh-item collection and deduplication keyed on
      `GitSpaceMetadata.checkout_key`, following `deduplicate_git_refresh_items`
      (`src/app/git_refresh.rs:136`).
- [x] 3.4 Implement `start_project_status_refresh_if_due(now)` and
      `project_status_refresh_deadline()` with an interval independent of, and longer than, the
      git-status interval. Run the provider work off the render path, as the git refresh does.
- [x] 3.5 Add a test asserting no refresh starts and no provider is spawned when no configured
      space row consumes either token, mirroring
      `due_git_refresh_does_not_start_without_sidebar_consumer`
      (`src/app/git_refresh.rs:324`).
- [x] 3.6 Add a test asserting a row containing only `proposals` enables only the proposals
      provider, and the mirror case for `beads`.
- [x] 3.7 Add a test asserting three workspaces sharing a `checkout_key` collapse to one refresh
      item, and a counter-test asserting two linked worktrees of one repository do not collapse.
- [x] 3.8 Run `cargo nextest run project_status_refresh` and paste the passing output.

## 4. Implement the two providers

- [x] 4.1 Implement the proposals provider: run `openspec list --json` with the checkout as the
      working directory, parse `changes[]`, and derive open and in-progress counts.
- [x] 4.2 Implement the beads provider: run `bd list --status open --json` with the checkout as
      the working directory, parse the array, and derive open, ready, and blocked counts.
- [x] 4.3 Give each provider a timeout and make every failure mode — binary absent, non-zero
      exit, timeout, non-JSON body, JSON of an unexpected shape — resolve to `None` for that
      provider only. No `unwrap()`; no user-visible error surface.
- [x] 4.4 Add parser tests over a captured `openspec list --json` body and a captured
      `bd list --status open --json` body, checked in as fixtures.
- [x] 4.5 Add failure tests for each of the five failure modes in 4.3, asserting `None` for the
      failing provider and an unaffected value for the other.
- [x] 4.6 Run `cargo nextest run project_status` and paste the passing output.

## 5. Wire the refresh into the runtime

- [x] 5.1 Call the due-check and report the deadline beside the existing git-refresh calls in
      `src/app/runtime.rs`, `src/server/headless.rs`, and
      `src/server/headless/scheduled_tasks.rs`. Do not merge the two deadlines into one.
- [x] 5.2 Add a test asserting a headless deadline can suppress the project-status timer,
      mirroring `headless_deadline_can_suppress_git_refresh_timer`
      (`src/app/git_refresh.rs:414`).
- [x] 5.3 Run `cargo nextest run` and paste the passing output.

## 6. Resolve the tokens in the sidebar

- [x] 6.1 In `src/ui/sidebar/tokens.rs`, extend `SpaceTokenContext` with the two optional count
      values and resolve the new tokens to their compact rendered forms (`op: 2o 1ip`,
      `bd: 3o 1r 0b`).
- [x] 6.2 In `src/ui/sidebar.rs`, pass the workspace's cached snapshot into `SpaceTokenContext`
      where `ahead_behind` is already passed.
- [x] 6.3 Add a render test asserting both tokens render with configured styles when both halves
      resolve.
- [x] 6.4 Add a render test asserting an absent half elides its token and its separator while the
      other half and every git token still render.
- [x] 6.5 Add a render test asserting a row containing only the two tokens is omitted when both
      halves are absent.
- [x] 6.6 Run `cargo nextest run sidebar` and paste the passing output.

## 7. Document and validate

- [x] 7.1 Document the two tokens, their rendered forms, their fail-open behaviour, and the
      `openspec`/`bd` optional dependency in `docs/next/website/src/content/docs/`. Do not edit
      the stable website docs or the root README.
- [x] 7.2 Add a `docs/next/CHANGELOG.md` entry.
- [x] 7.3 Run `just check` and paste the passing output.
- [x] 7.4 Manually verify against a live server: configure a `[ui.sidebar.spaces]` row containing
      both tokens, open one space on this repo (proposals present, no beads database) and one on
      a repo with a beads database, and confirm each renders the available half and elides the
      other. Paste the observed rows.
