## Why

Base commit: **`81fd2237`** (branch `dev`).

`docs/next/socket-auth-capability-spike.md` is staged for the next release, and twice delegates
its substance to a file that does not exist and never did.

Line 76-78:

> The full 91-row table lives in
> `openspec/changes/spike-socket-auth-capability/design.md` § Section B — this
> doc does not reproduce it.

Line 112-114:

> Full detail, evidence citations, and the reconciliation with `c000681f` and
> `spike-plugin-install-api`:
> `openspec/changes/spike-socket-auth-capability/design.md`.

That path is dead three ways. `openspec/changes/spike-socket-auth-capability/` is absent from the
worktree; it is not in `openspec/changes/archive/` under any date prefix; and
`git log --all --name-status -- 'openspec/changes/spike-socket-auth-capability/**'` shows the
directory only ever contained `proposal.md` and `tasks.md`. `design.md` was never tracked. The
directory was deleted in `80e25966` — the same commit that created the citing doc. So the spike
shipped its pointer and removed its target in one motion, and the 91-row tiering table it
describes exists nowhere in the repository's history.

This is not a broken link to recoverable content. It is a doc that deliberately did not reproduce
its own payload, on the promise of an artifact that was already gone.

The count has since drifted, which is how the loss becomes load-bearing. The doc claims 91
client-facing `Method` variants tiered 28 read-only / 54 mutate / 9 exec. Today
`src/api/schema.rs`'s `Method` enum has 95 variants, 92 carrying `#[serde(rename = ...)]`. The 3
without a rename are exactly the three the doc names as internal re-dispatch targets and excludes
(`PaneGraphicsStreamSet`, `PaneGraphicsStreamOpen`, `PaneGraphicsStreamClose` —
`src/api/schema.rs:187,190,193`). So the client-facing set is now **92**, not 91, and the
28/54/9 breakdown no longer sums to the inventory.

`git diff 80e25966 HEAD -- src/api/schema.rs` shows exactly one added variant: `ServerConfigEdit`,
wire name `server.config.edit` (`src/api/schema.rs:58-59`), landed in `a94ebedf` ("feat: open the
resolved server config in an editor pane"). The doc was accurate when written; one method has
been added and is untiered.

That method is not a benign addition. The doc's own stated criterion for the exec tier is that a
method spawns a process the caller influences — `agent.start` is exec because
`AgentStartParams.args: Vec<String>` is attacker-controllable argv. Opening the resolved config in
an editor pane spawns an editor process, which puts `server.config.edit` in or adjacent to the
highest tier rather than the read-only tail. A future enforcement layer built from this doc would
inherit a silent gap at exactly the tier that matters, and there is no surviving table against
which anyone could notice.

Restoring the table into the doc that cites it also removes the failure mode that produced this:
a staged release doc whose correctness depends on an untracked sibling in a directory that gets
archived or deleted on its own schedule.

## What Changes

- Regenerate the method capability tiering table from `src/api/schema.rs` and inline it into
  `docs/next/socket-auth-capability-spike.md` as a new `## Method capability tiering (full table)`
  section, replacing the "this doc does not reproduce it" deferral at line 76-78.
- Tier `server.config.edit` explicitly, with a one-line rationale naming the editor-process spawn,
  and update the summary counts from 91/28/54/9 to the recounted values.
- Remove both dead `openspec/changes/spike-socket-auth-capability/design.md` pointers (lines 77
  and 112-114). The line 112-114 block promises "full detail, evidence citations, and the
  reconciliation with `c000681f` and `spike-plugin-install-api`" — the reconciliation paragraph
  already exists in this doc under `## Open follow-ups` ("Tiering mismatch with
  `spike-plugin-install-api`, flagged not absorbed"), so the pointer is replaced by a cross
  reference to that section rather than deleted outright.
- Add `scripts/socket_capability_tiering_check.py` asserting that the table's client-facing row
  count equals the number of `#[serde(rename = ...)]` variants in `Method`, so the count cannot
  drift silently again. Wire it into `just release-docs-check` beside the existing checks.
- Leave the shipped peer-uid code untouched. This change is documentation plus one maintenance
  script; no runtime, protocol, or API behaviour moves.

Two claims in the doc were re-verified against `81fd2237` and are still accurate — do not "fix"
them: `peer_uid_authorized` still has its unix and Windows arms (`src/ipc.rs:228,246`) and is
still wired into both accept paths (`src/api/server.rs:118`, `src/server/client_accept.rs:42`),
and `PlatformCapabilities::peer_credential_check` (`src/platform/mod.rs:54`, set at `:63`) still
has zero readers, exactly as the doc's follow-up states. The doc's cited line numbers for the two
accept sites have drifted (it says `src/api/server.rs:144` and `src/server/client_accept.rs:49`);
refresh them while editing.

## Acceptance

- `docs/next/socket-auth-capability-spike.md` contains the full tiering table inline and no
  reference to `openspec/changes/spike-socket-auth-capability/`.
- The table's client-facing row count matches the live `Method` enum, and `server.config.edit`
  carries an explicit tier and rationale.
- `docs-sweep --json` reports verdict `verified` for the page with zero `dangling_ref` findings.
- `just release-docs-check` passes with the new tiering check wired in.
