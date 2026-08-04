## Context

GitHub reports `dev` as this fork's default branch, `README.md` calls `origin/dev` authoritative, and `AGENTS.md` directs maintainers to validate and push the current integration branch directly. The validation workflows still accept pushes only to `master` (plus the legacy `windows` CI branch), so the direct path lacks independent Linux, macOS, Windows, maintenance, Website, and Nix evidence.

Stable and preview publication still use `master`. The `just release` recipe first invokes `release-prepare`, which can rewrite release documents and versions and create a commit on any clean branch; only the subsequent `release-publish` recipe checks for `master`. The approved policy is therefore a two-branch lifecycle, not a global rename: development integrates on `dev`, a verified candidate is fast-forwarded to `master`, and stable release preparation and publication run on `master`.

Three active OpenSpec entries predate the current schema. They contain useful decisions and tasks, but no delta specs and no checkbox tasks. OpenSpec consequently rejects all three and reports each as `0/0`. The plugin lifecycle implementation and decision note are already present; the user's approval of finding 3 accepts the documented conservative outcome: keep `plugin outdated` manual and do not build automatic updates yet.

## Goals / Non-Goals

**Goals:**

- Give direct `dev` pushes the repository's existing independent validation.
- Prevent release preparation from mutating the wrong branch.
- Make the `dev` to `master` handoff explicit, fetched, and fast-forward-only.
- Restore strictly valid, parser-visible OpenSpec state without duplicating existing owners.
- Add lightweight regression tests using the existing stdlib maintenance-test style.

**Non-Goals:**

- Releasing or publishing previews directly from `dev`.
- Changing pull-request target policy, branch protection, deployment permissions, preview selection, release-tag reachability, or release-workflow artifact pinning.
- Expanding Nix/Website path inputs beyond the currently approved branch-alignment work.
- Implementing Windows coverage, the community-agent submission path, plugin auto-update, background plugin polling, or a TUI plugin-update surface.
- Dispatching GitHub workflows, creating tags, promoting branches, or publishing anything during implementation.

## Decisions

### Preserve a two-branch integration/publication model

`dev` remains the default integration branch; `master` remains the stable publication branch because release, preview, website publication, and issue-labeling machinery already share that contract. Rejected: replacing every `master` reference with `dev`, which would silently broaden this change into release and deployment migration.

Before release, the maintainer fetches both branches and tags, checks out `master`, and fast-forwards it to the exact fetched `origin/dev`. If fast-forward is impossible, the procedure stops so publication commits or other divergence can be integrated into `dev` and revalidated. Rejected: a force push, an automatic merge commit, or pushing an arbitrary local `HEAD` as the promotion mechanism.

### Reuse one fail-fast release branch preflight

Add a non-mutating `release-branch-check` recipe that requires `master`, and make both `release-prepare` and `release-publish` depend on it. This executes before either recipe body, preventing changelog/version edits and release commits on `dev` while retaining the existing publish checks. Rejected: retaining duplicate late guards or performing an automatic checkout inside the release recipe.

### Extend validation triggers without changing their work

Add `dev` to the existing push branch lists. Make CI's conventional-commit and maintenance predicates accept both `dev` and `master`; the matrix and Windows package jobs already run for every accepted push. Preserve pull-request semantics and path filters. Add the existing `scripts.test_shepherd_release_boundary` suite to the CI maintenance job because `just check` already runs it but CI currently does not.

### Verify workflow contracts with stdlib structural tests

Extend `scripts/test_shepherd_release_boundary.py`. Tests inspect minimal stable markers and ordering rather than parse YAML with a new dependency or execute stateful release commands. A separate manual check may invoke only the new non-mutating branch preflight on `dev`, where failure is the expected result. Rejected: PyYAML, workflow dispatch, real tag creation, or a black-box `release-publish` test.

### Repair existing OpenSpec owners in place

For Windows and remote-agent work, preserve the existing decisions and incomplete outcomes while adding current proposal headings, delta specs, design artifacts, and checkbox tasks with exact validation. They remain active.

For plugin lifecycle, retain the delivered manual drift behavior and migration regression, update the decision-note status from pending to accepted by the user's approval, add a delta spec and completed checkbox tasks, validate, then archive normally. Automatic update and unattended polling remain explicit future features rather than unfinished tasks in the completed conservative increment.

Rejected: creating duplicate proposals; archiving the live Windows/remote owners merely because their legacy format is invalid; using `--no-validate`; or merging rejected full remote string-identity behavior into canonical specs.

## Risks / Trade-offs

- **More Actions usage on direct pushes** → Preserve path filters and existing concurrency cancellation; do not add jobs or broaden PR triggers.
- **Duplicate validation when a PR is followed by a `dev` push** → Accept the independent post-integration run because it validates the authoritative commit SHA.
- **`master` may contain publication commits absent from `dev`** → Require fast-forward-only promotion and stop on divergence; guidance explains that `master` must be integrated back into `dev` and revalidated rather than overwritten.
- **Static workflow tests can become overly textual** → Assert only branch sets, named suite inclusion, recipe dependencies, guard messages, and relative ordering; keep GitHub's actual post-push run as the operational proof.
- **Legacy OpenSpec repair can accidentally imply implementation** → Preserve checked versus unchecked state explicitly, validate each change independently, and archive only the completed plugin change.

## Migration Plan

1. Add failing structural tests for the approved branch and planning contracts.
2. Update workflow triggers, CI maintenance coverage, release preflight, and maintainer guidance.
3. Repair the three active OpenSpec changes and archive only plugin lifecycle after strict validation.
4. Run strict validation for all specs/changes, focused maintenance tests, available YAML lint, and `just check`.
5. After the reviewed implementation is committed and pushed with user-approved commit text, observe the `dev` CI run to confirm GitHub schedules the expected jobs.

Rollback is a normal revert of the workflow/release/planning commit. No persisted product data, tag, deployment, or external branch setting is changed by implementation.

## Open Questions

None for this approved scope. Broader release-SHA hardening, preview-source policy, additional Nix inputs, and publication-commit synchronization remain separately reviewable work.
