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

## Open Questions

None. Exact Rust type names may follow existing schema conventions, but the two-dimensional effective/reporter semantics are fixed.
