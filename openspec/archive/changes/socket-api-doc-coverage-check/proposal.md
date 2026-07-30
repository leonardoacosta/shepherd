# Add a socket-API method-coverage check

Base commit: `1de05dc2` · Route: proposal · Effort: S · Confidence: HIGH · Category: docs / DX

## Why

Adjacent follow-on resolving advisory finding DOCS-02, and a natural rider on
`ci-maintenance-gate` (same CI job) — especially since we are about to add
methods (`pane.output` subscription, `plugin.outdated`, possibly `plugin.install`).

The machine-readable schema is protected but the human-readable page is not:

- `src/api/schema.rs:45-238` — the `Method` enum declares ~90 wire method names.
- `src/api/schema/tests.rs:~159` asserts `docs/next/api/herdr-api.schema.json`
  matches the generated schema, and `tests/cli/surface.rs:~538` covers
  `herdr api schema` — so the generated artifact stays in sync.
- But `website/src/content/docs/socket-api.mdx` (the page users read) has no such
  check. Two methods are already missing from it: `server.live_handoff` and
  `workspace.move_block`. `server.live_handoff` is the method that performs live
  server replacement during self-update.
- `scripts/config_reference_check.py` is the exact template: it diffs the config
  model against the reference doc. There is no socket-API analogue.

## What changes

Add `scripts/socket_api_reference_check.py`, modeled on
`config_reference_check.py`, that reads method names from the already-generated
`docs/next/api/herdr-api.schema.json` and asserts each appears in
`socket-api.mdx`, with an explicit skip list for `#[schemars(skip)]` internals
(e.g. `pane.graphics.stream` and the `serde(skip)` stream-open/close variants).
Wire it into `just release-docs-check` and the `maintenance` CI job introduced by
`ci-maintenance-gate`.

## Acceptance

- `python3 scripts/socket_api_reference_check.py` passes on a head where every
  non-skipped method is documented — which means documenting the two currently
  missing methods (`server.live_handoff`, `workspace.move_block`) as part of this
  change.
- The check is referenced in `just release-docs-check` and the maintenance CI job.
- Adding a new method without documenting it makes the check fail.

## Out of scope

- Params/response-shape coverage — this checks method-name presence only (same
  scope discipline as `config_reference_check.py`). Deeper coverage can follow.
- The stable-vs-next docs split beyond pointing the check at the generated schema.

## Dependencies

- Composes with `ci-maintenance-gate` (provides the CI job slot). Can land
  independently; if that job doesn't exist yet, wire into `release-docs-check`
  now and the CI job when it lands.
