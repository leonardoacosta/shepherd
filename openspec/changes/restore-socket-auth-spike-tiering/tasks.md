## 1. Confirm the drift before editing

- [ ] 1.1 Confirm the base commit still matches. Run
  `git rev-parse --short HEAD` and expect `81fd2237`, or confirm the excerpts below still hold.
  If `docs/next/socket-auth-capability-spike.md` no longer says "this doc does not reproduce it"
  at line 76-78, or `openspec/changes/spike-socket-auth-capability/` now exists, **STOP and report
  the drift** — do not improvise.
- [ ] 1.2 Recount the client-facing method set. Run
  `awk '/^pub enum Method \{/{f=1;next} f&&/^\}/{exit} f' src/api/schema.rs | grep -c '#\[serde(rename'`
  and expect `92`. Then run the same awk piped to
  `grep -vE '^\s*(//|#\[)' | grep -cE '^\s{4}[A-Z]'` and expect `95`. The difference of 3 must be
  exactly `PaneGraphicsStreamSet`, `PaneGraphicsStreamOpen`, `PaneGraphicsStreamClose`; confirm
  with `grep -n 'PaneGraphicsStream' src/api/schema.rs`. If the numbers differ, use the live
  counts and note the delta in this task.

## 2. Restore the tiering table into the doc

- [ ] 2.1 Derive the tier for every `#[serde(rename = ...)]` variant in `Method`
  (`src/api/schema.rs:45-238`) using the doc's own stated criteria: read-only returns state
  without mutating; mutate makes a bounded state change; exec spawns or influences a process.
  Apply the doc's two named precedents verbatim — `agent.start` is exec because
  `AgentStartParams.args` is caller-controlled argv, and `layout.apply` is exec because
  `LayoutApplyParams.root` recurses into `LayoutPane.command`.
- [ ] 2.2 Tier `server.config.edit` (`src/api/schema.rs:58-59`) and record the rationale in one
  line. It opens the resolved config in an editor pane, which spawns an editor process; tier it by
  the same rule that makes `agent.start` exec, and say so explicitly rather than leaving the call
  implicit.
- [ ] 2.3 Replace lines 68-78 of `docs/next/socket-auth-capability-spike.md` with a
  `## Method capability tiering (full table)` section carrying the inline table (one row per
  client-facing method: wire name, tier, and a rationale column populated for every exec row and
  for any non-obvious mutate row). Keep the existing `agent.start` and `layout.apply` call-outs.
  Update the summary counts to the recounted values from 1.2.
- [ ] 2.4 Replace the trailing pointer at lines 112-114 with a cross reference to this doc's own
  `## Open follow-ups` section, which already carries the `spike-plugin-install-api`
  reconciliation. Refresh the two drifted line citations in `## What shipped in this spike` to
  `src/api/server.rs:118` and `src/server/client_accept.rs:42`.
- [ ] 2.5 Verify no dead pointer survives. Run
  `grep -c 'spike-socket-auth-capability' docs/next/socket-auth-capability-spike.md` and expect
  `0` (the exit status will be 1 on zero matches, which is the expected result here).

## 3. Make the count self-checking

- [ ] 3.1 Add `scripts/socket_capability_tiering_check.py` asserting that the number of table rows
  in the doc's tiering section equals the `#[serde(rename` count in `Method`, failing with the
  two numbers on mismatch. Model it on `scripts/socket_api_reference_check.py`, which already
  validates `docs/next/website/src/content/docs/socket-api.mdx` against
  `docs/next/api/shepherd-api.schema.json` — same shape, same failure style, same
  `SCHEMA_PATH`/`DOC_PATH` module constants.
- [ ] 3.2 Add `scripts/test_socket_capability_tiering_check.py` covering the matching case and the
  drift case. Run `python3 -m unittest scripts.test_socket_capability_tiering_check` and expect all
  tests to pass.
- [ ] 3.3 Wire `python3 scripts/socket_capability_tiering_check.py` into the `release-docs-check`
  recipe in `justfile:84-99`, beside the existing `socket_api_reference_check.py` line. Run
  `just release-docs-check` and expect exit 0.

## 4. Verify and hand off

- [ ] 4.1 Re-run the docs sweep scoped to this page:
  `docs-sweep --json | jq '.docs[] | select(.path == "docs/next/socket-auth-capability-spike.md")'`
  and expect `verdict` `verified` with no `dangling_ref` finding (a `frontmatter` advisory is
  expected and correct for this repo — do not add `status:`/`updated:` frontmatter, see the run
  note below).
- [ ] 4.2 Run `just check` and expect all repository checks to pass. Propose the commit message and
  get alignment before committing, per `AGENTS.md`.

> Note for the executor: this repo's `docs/` tree is a release-staging tree, not the operational-docs
> canon tree. `docs-sweep` running with `config: defaults` reports `frontmatter` advisories on all
> eight scanned pages. Do **not** act on them. Adding `status:`/`updated:` frontmatter to
> `docs/next/CHANGELOG.md` would break `justfile:90`, which requires it to be byte-identical to root
> `CHANGELOG.md`.
