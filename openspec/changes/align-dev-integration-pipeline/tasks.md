## 1. Fence the approved workflow change

- [ ] 1.1 Refresh `HEAD`, `origin/dev`, `origin/master`, GitHub's default branch, dirty state, active OpenSpec changes, and touched-path overlap; complete the project-required read-only roundtable covering CI triggers, release safety, verification, and OpenSpec disposition. Run `git status --short && git rev-parse HEAD origin/dev origin/master && openspec list --json`; expected result: only this proposal's authoring paths are dirty, the approved base remains current or is deliberately restamped, and no competing change owns the workflow/release paths.
  - touches: none (read-only fencing and review)
  - depends on: []

## 2. Protect direct integration and release preparation

- [ ] 2.1 Add failing structural regression tests for `dev` push validation, CI maintenance inclusion of `scripts.test_shepherd_release_boundary`, the shared release-branch preflight dependency/order, and the documented fast-forward-only promotion. Run `python3 -m unittest scripts.test_shepherd_release_boundary`; expected result before implementation: only the new assertions fail at the missing branch/preflight contracts.
  - touches: `scripts/test_shepherd_release_boundary.py`
  - depends on: 1.1

- [ ] 2.2 Add `dev` to the existing CI, Website, and Nix push branch sets; make CI's conventional-commit and maintenance predicates accept `dev`; and add the release-boundary test module to CI maintenance without changing current PR semantics, path filters, job bodies, deployments, or release workflows. Run `python3 -m unittest scripts.test_shepherd_release_boundary`; expected result: the workflow branch and maintenance contracts pass.
  - touches: `.github/workflows/ci.yml`, `.github/workflows/website.yml`, `.github/workflows/nix.yml`
  - depends on: 2.1

- [ ] 2.3 Add one non-mutating `release-branch-check` recipe, make both release phases depend on it before their bodies execute, remove the redundant late branch check, and document fetched fast-forward-only `dev` to `master` promotion plus the divergence stop condition. Run `python3 -m unittest scripts.test_shepherd_release_boundary && before="$(git status --porcelain=v1)" && if just release-branch-check; then exit 1; fi && test "$before" = "$(git status --porcelain=v1)"`; expected result: structural tests pass, the preflight rejects the current `dev` checkout, and it changes no tracked or untracked path.
  - touches: `justfile`, `AGENTS.md`, `scripts/test_shepherd_release_boundary.py`
  - depends on: 2.1

## 3. Reconcile the authoritative OpenSpec queue

- [ ] 3.1 Refresh `windows-test-coverage` into current proposal/spec/design/task shape while preserving the real Windows measurement prerequisite, filtered-suite evidence, Windows VM boundary, and incomplete implementation state. Run `openspec validate windows-test-coverage --strict --no-interactive && openspec status --change windows-test-coverage`; expected result: strict validation passes and parser-visible unchecked tasks describe measurement, triage, gate expansion, and verification.
  - touches: `openspec/changes/windows-test-coverage/`
  - depends on: 1.1

- [ ] 3.2 Refresh `spike-remote-agent-catalog` into current proposal/spec/design/task shape while preserving the chosen community-submission path, the rejected full string-identity architecture, trust review requirements, and incomplete documentation/fixture work. Run `openspec validate spike-remote-agent-catalog --strict --no-interactive && openspec status --change spike-remote-agent-catalog`; expected result: strict validation passes and parser-visible tasks truthfully separate completed research from incomplete accepted-path work.
  - touches: `openspec/changes/spike-remote-agent-catalog/`
  - depends on: 1.1

- [ ] 3.3 Record the user-approved conservative plugin lifecycle decision, refresh `spike-plugin-lifecycle` into valid proposal/spec/design/task shape with every delivered acceptance task checked, run `openspec validate spike-plugin-lifecycle --strict --no-interactive`, and archive it normally with `openspec archive spike-plugin-lifecycle -y`; expected result: validation and archive exit 0, the change leaves the active list, and canonical `plugin-upstream-drift` requirements describe the shipped manual command without automatic updates.
  - touches: `openspec/changes/spike-plugin-lifecycle/`, `docs/next/plugin-lifecycle-spike.md`
  - depends on: 1.1

## 4. Verify and close

- [ ] 4.1 Run `openspec validate --all --strict --no-interactive && python3 -m unittest scripts.test_shepherd_release_boundary && yamllint -d relaxed .github/workflows/ci.yml .github/workflows/website.yml .github/workflows/nix.yml && git diff --check`; expected result: every command exits 0, every active change/spec is valid, the focused regressions pass, and workflow YAML has no error-level lint finding.
  - touches: none (read-only validation)
  - depends on: 2.2, 2.3, 3.1, 3.2, 3.3

- [ ] 4.2 Run `just check`; expected result: formatting, all nextest tests, Windows-target clippy, integration assets, plugin marketplace, generated client, and maintenance suites exit 0 on the final bytes.
  - touches: none (read-only validation)
  - depends on: 4.1

- [ ] 4.3 Recheck `git status --short`, proposal touches, `origin/dev`, active OpenSpec ownership, and final artifact validity; then archive `align-dev-integration-pipeline` normally after approved persistence. Expected result: only approved implementation plus apply-owned task/archive changes are present, no unrelated work is staged, and the canonical maintainer integration lifecycle spec matches the shipped workflow.
  - touches: none (read-only closeout and apply-owned OpenSpec state)
  - depends on: 4.2
