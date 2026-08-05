## Context

`TerminalState` already arbitrates screen fallback, full-lifecycle hook authority, mixed/custom reports, and visible blocker overrides. Public `PaneInfo` and `AgentInfo` expose only a boolean indicating that screen detection was skipped, so clients cannot explain the evidence behind a status or aggregate current reporters accurately.

The projection must respect the runtime/client boundary: evidence and authority are server facts, while wording, counts, and colors remain client presentation. A mixed report may remain active while the effective state comes from a newer screen blocker.

## Goals / Non-Goals

**Goals:**

- Give `PaneInfo` and `AgentInfo` one identical typed projection for the same terminal.
- Represent effective evidence and active reporter observation without implying health or freshness.
- Preserve compatibility for `screen_detection_skipped` and existing session identity fields.
- Support exact-source integration aggregation without guessing custom reporters.

**Non-Goals:**

- Report-age heartbeats, green/yellow/red health, or install-state claims.
- Exposing session identifiers in aggregate Settings copy.
- Changing authority arbitration or lifecycle behavior.
- Adding a TUI-specific socket method.

## Decisions

### Separate effective evidence from reporter observation

Expose a shared structure with an `effective` value (`screen` or `reported { source, authority }`) and an optional `reporter` value (`source`, `authority`). A full-lifecycle report normally populates both. A mixed report overridden by a visible screen blocker reports `effective = screen` while retaining the mixed reporter observation. Expired or cleared reports disappear from the projection.

Rejected: one `Reported { authority: Mixed }` value for every active hook, because it would mislabel screen-overridden states.

### Derive both API models from the terminal boundary

Add one terminal projection helper and have both `PaneInfo` and `AgentInfo` consume it. Keep `screen_detection_skipped` derived from effective exclusive reported evidence so old clients retain its meaning.

Rejected: duplicating arbitration checks in each API handler.

### Aggregate only canonical registered sources

Settings may count current reporters and session identities by exact registered integration source. Unknown/custom sources remain visible only as generic runtime facts and are never attributed to a named integration. Absence is described as not currently observed, not failed.

## Risks / Trade-offs

- **Projection drift** → characterize PaneInfo/AgentInfo parity from one terminal and test screen-overridden mixed reports.
- **Accidental health contract** → omit timestamps and freshness statuses from the public projection.
- **Wire compatibility** → compare protocol source 19 with the latest release before editing fixtures; update generated consumers together.
- **Custom-source attribution** → require exact source/agent matches and add unknown-source tests.

## Migration Plan

1. Add types and terminal projection tests while retaining compatibility fields.
2. Wire both API projections and regenerate schema/TypeScript clients.
3. Add descriptive integration aggregation and next docs.
4. Roll back by removing optional fields and client consumers; no persisted data migration is required.

## Implementation survey (verified 2026-08-04, against `dev` at 99166485)

Source-of-truth locations confirmed by reading current source, so the implementation does not
re-derive them or duplicate existing arbitration:

- Arbitration already exists in `TerminalState` (`src/terminal/state.rs`). The projection reads it;
  it does not add rules.
  - `hook_authority_is_effective` (~1695) — a full-lifecycle authority counts only when its parsed
    agent equals `detected_agent` and there is no `recent_agent_process_exit`. A non-full-lifecycle
    (mixed) authority is unconditionally "effective" at this layer.
  - `live_full_lifecycle_hook_authority` (~1754) — effective AND full-lifecycle. Public alias:
    `full_lifecycle_hook_authority_active` (~1737).
  - `visible_blocker_overrides_hook` (~1741) — only reachable when NOT full-lifecycle. This is
    exactly the spec's "mixed report overridden by screen evidence" case: effective becomes
    `screen` while the mixed reporter observation is retained.
  - `HookAuthority` (~18) already carries `source`, `agent_label`, `state`, `reported_at`, and
    `session_ref` — the reporter fields need no new plumbing, and `session_ref` is what must stay
    separate from reporter observation.
- Classification helpers to REUSE, not reinvent: `full_lifecycle_hook_authority(source, agent_label)`
  and `session_identity_only_integration(source, agent_label)` (`src/detect/mod.rs` ~283 / ~295).
  The full-lifecycle set is a fixed match list; `hermes` is the session-identity-only case.
- Compatibility: `screen_detection_skipped` is currently derived at `src/app/agents.rs:381` from
  `full_lifecycle_hook_authority_active()`. Deriving it from "effective is reported with
  exclusive_lifecycle authority" is therefore equivalent by construction — no behavior change, and
  no parallel bool should survive.
- `src/api/schema/agents.rs:200` declares `screen_detection_skipped` on `AgentInfo`; `PaneInfo`
  (`src/api/schema/panes.rs`) has no equivalent today, so the shared projection is what first puts
  the two models on one contract.
- NOT a defect: `src/app/api/agents.rs:188` hard-codes `"screen_detection_skipped": true`, but the
  enclosing block at line 174 is already guarded by `full_lifecycle_hook_authority_active()`, so the
  literal is correct in context. Do not "fix" it.

Protocol: latest release tag `v0.7.5` carries `PROTOCOL_VERSION = 17`; current source is `19`.
Source is already ahead, so task 4.1 must NOT bump it.

## Open Questions

None. Exact Rust type names may follow existing schema conventions, but the two-dimensional effective/reporter semantics are fixed.
