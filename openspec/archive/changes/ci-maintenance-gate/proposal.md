# Gate release-critical maintenance checks on PR CI

Base commit: `1de05dc2` · Route: proposal · Effort: S · Confidence: HIGH · Category: DX / tooling

## Why

Resolves advisory finding TESTS-03 (audit against `1de05dc2`).

Several validators that block a release are not run on PRs, so drift surfaces
mid-release instead of at PR time:

- `justfile:27-30` — `just ci` (what PR CI runs) is lint + `cargo nextest` + two
  bun suites. It does **not** run the nine `python3 -m unittest scripts.test_*`
  maintenance modules — those live only in `just test` and `just check`.
- `.github/workflows/ci.yml:139` — the PR job invokes `just ci '<filter>'`.
- `scripts/config_reference_check.py`, `scripts/agent_detection_manifest_check.py`,
  and `scripts/docs_translation_parity.py` appear in **no workflow** — only in
  `just release-docs-check` (`justfile:79+`).
- `.github/workflows/preview.yml` is the only workflow running `just check`, and
  it triggers on `workflow_dispatch` only.

Consequence: a PR adding a field to `src/config/model.rs` (a high-churn file)
passes CI green, then fails `just release-docs-check` when the maintainer is
mid-release. Same for any edit to `src/detect/manifests/` or to a
`docs/next/website/**` page missing its `ja`/`zh-cn` counterpart.

## What changes

Add a fast `maintenance` job to `.github/workflows/ci.yml` (ubuntu, Python only —
no Rust/Zig build needed) that runs, on every PR:

- the nine `scripts.test_*` unittest modules,
- `config_reference_check.py`,
- `agent_detection_manifest_check.py`,
- `docs_translation_parity.py` on both doc roots.

Roughly a minute of CI, moving release-blocking failures to PR time.

## Acceptance

- The new job is green on the current `dev`/`master` head. If it surfaces a
  pre-existing failure, that failure is triaged and either fixed in this change
  or explicitly recorded as a follow-up before the job is made required.
- The job runs on `pull_request` (and push to the default branch).
- No change to `just ci` behavior for the Rust jobs.

## Out of scope

- Restructuring `justfile` recipes — only the CI workflow gains a job.
- Adding new checks; this only relocates existing validators earlier.
