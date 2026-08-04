## Why

Shepherd's fork now treats `dev` as its authoritative integration branch and asks maintainers to push that branch directly, but the repository's validation and release guardrails still assume ordinary work lands on `master`. As a result, `dev` pushes receive no independent CI and `just release` can mutate and commit on `dev` before its later publish step refuses the branch.

The active OpenSpec queue has the same workflow drift: all three legacy entries fail strict validation and expose zero parser-visible tasks, while `spike-plugin-lifecycle` has already shipped its approved conservative outcome. Align the branch lifecycle and restore one truthful executable planning queue before more direct integration work lands.

## What Changes

- Run the existing CI, maintenance, Website, and Nix validation workflows on qualifying pushes to `dev` while preserving their current pull-request, `master`, `windows`, and path-filter behavior.
- Make the release branch contract fail closed before `release-prepare` changes files or creates a commit, and document the explicit fast-forward-only `dev` to `master` promotion sequence used before a stable release.
- Add maintenance tests that pin the integration-workflow triggers and release preflight ordering without dispatching workflows, creating tags, or pushing branches.
- Repair `windows-test-coverage` and `spike-remote-agent-catalog` into current, strictly valid OpenSpec changes with delta specs and parser-visible tasks, preserving their existing approved scope and ownership.
- Record the user's approval of the conservative manual-only plugin drift outcome, make `spike-plugin-lifecycle` strictly valid, and archive it normally so the shipped `plugin outdated` contract becomes a canonical specification.

## Capabilities

### New Capabilities

- `maintainer-integration-lifecycle`: Validation, release-branch preflight, promotion, and planning-state requirements for Shepherd's `dev` integration and `master` publication branches.

### Modified Capabilities

None.

## Impact

- Workflows: `.github/workflows/ci.yml`, `.github/workflows/website.yml`, and `.github/workflows/nix.yml` gain `dev` push coverage; no deployment, preview, release, issue-labeling, or branch-protection behavior changes.
- Release tooling and guidance: `justfile`, `AGENTS.md`, and `scripts/test_shepherd_release_boundary.py` define and verify a fail-fast `master` release preflight and explicit promotion procedure.
- Planning state: the three existing active OpenSpec entries are repaired in place; only the completed plugin lifecycle entry is archived. Windows coverage and remote-agent contribution work remain active and unimplemented.
- Dependencies and product behavior: no application dependency, wire protocol, persisted data, TUI behavior, stable documentation, GitHub branch setting, tag, release, or external deployment changes.
- Base and baseline:
  - base-commit: shepherd@a298a51a458c339145d47d8a999291ff8b39f060
  - dirty-baseline: clean before `openspec new change align-dev-integration-pipeline`
- Issue linkage: inapplicable because this repository has no Beads database and the approved findings are fork-local workflow work.
- touches: `.github/workflows/ci.yml`, `.github/workflows/website.yml`, `.github/workflows/nix.yml`, `justfile`, `AGENTS.md`, `scripts/test_shepherd_release_boundary.py`, `openspec/changes/spike-plugin-lifecycle/`, `docs/next/plugin-lifecycle-spike.md`, `openspec/changes/windows-test-coverage/`, `openspec/changes/spike-remote-agent-catalog/`

## Preconditions

- Recheck that GitHub's default branch and `origin/HEAD` remain `dev`, `origin/master` remains an ancestor of the approved integration head, and the workflow/release predicates still match the cited baseline before implementation.
- Preserve the approved branch policy: `dev` is the direct integration branch and `master` remains the stable publication branch. A change that instead releases directly from `dev` requires a new scope covering preview, release, manifest publication, issue labeling, and branch protection.
- Preserve current pull-request targeting and current workflow path-filter behavior. Additional Nix input coverage, Website trigger cleanup, immutable release-SHA redesign, or publication-commit mirroring are evidence-backed follow-ups but outside this approved change.
- The project-required release-risk roundtable must complete before workflow or release edits, with unresolved findings either incorporated within this scope or explicitly excluded.

## Decisions

- Keep `dev` as the direct integration branch and `master` as the stable publication branch. Promote only by fetched fast-forward before running release preparation on `master`; stop on divergence. Rejected: migrating every preview/release/publication surface to `dev` in this change.
- Reuse the current validation jobs and path filters, adding only `dev` push eligibility and the omitted release-boundary maintenance suite. Rejected: changing PR targeting, adding jobs, or broadening unrelated input filters.
- Use one non-mutating `release-branch-check` prerequisite for both release phases. Rejected: a late duplicate guard or an automatic branch checkout inside a release recipe.
- Repair the Windows and remote-agent owners in place and keep them active; archive only the completed, user-accepted conservative plugin lifecycle change. Rejected: duplicate proposals, `--no-validate`, or treating invalid legacy formatting as proof that live work is finished.

## Done Means

- A qualifying push to `dev` schedules the same existing integration checks that protect `master`, including conventional-commit and maintenance jobs; qualifying Website and Nix changes also schedule their existing validation workflows.
- `release-prepare` and `release-publish` both refuse a non-`master` checkout through one reusable preflight before release files, commits, branch pushes, or tags can change.
- Maintainer guidance names `dev` as integration, `master` as publication, uses a fetched fast-forward-only promotion, and stops rather than rewriting history when `master` is not an ancestor of `dev`.
- Static maintenance tests cover the branch triggers, maintenance-suite inclusion, release-preflight dependency/order, and promotion guidance; the tests do not dispatch Actions or exercise destructive release effects.
- `spike-plugin-lifecycle` is strictly valid and archived normally with its shipped manual drift contract in canonical specs; the two remaining active changes are strictly valid and expose their real incomplete tasks through `openspec list`.
- `openspec validate --all --strict --no-interactive`, focused maintenance tests, workflow lint available in the environment, `git diff --check`, and `just check` all pass on the final tree.

## Testing

- Run `python3 -m unittest scripts.test_shepherd_release_boundary`; expected result: integration branch sets, CI maintenance coverage, release preflight order, and promotion guidance contracts pass without external effects.
- Run `before="$(git status --porcelain=v1)" && if just release-branch-check; then exit 1; fi && test "$before" = "$(git status --porcelain=v1)"` from `dev`; expected result: the preflight refuses the branch and leaves repository state byte-for-byte unchanged.
- Run `openspec validate --all --strict --no-interactive`; expected result: every active change and canonical specification passes, the completed plugin change is absent from the active list, and remaining tasks are parser-visible.
- Run `yamllint -d relaxed .github/workflows/ci.yml .github/workflows/website.yml .github/workflows/nix.yml && git diff --check`; expected result: no YAML error or whitespace error.
- Run `just check`; expected result: formatting, Rust tests, maintenance tests, Windows-target lint, integration assets, marketplace tests, and generated client checks pass.
