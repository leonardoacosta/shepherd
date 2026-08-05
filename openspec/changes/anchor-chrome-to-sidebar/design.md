## Context

`surface-configurable-chrome` delivered the topbar in two layers that did not agree with each
other. The token layer extends the Agent sidebar. The render layer does not.

The token layer is already correct. `src/ui/topbar.rs` resolves rows through the sidebar's own
functions — `sidebar::tokens::agent_rows_from` and `sidebar::resolved_token_spans` — and
`src/app/state.rs:1779` types the resolved rows as `Vec<Vec<AgentSidebarToken>>`. There is one
token vocabulary and the topbar shares it. This change does not touch that layer.

The render layer treats the topbar as the sidebar's parent-level sibling. `src/ui.rs:242-251`
removes a full-width band from the whole frame, then splits what remains:

```rust
let [topbar_area, body_area] =
    Layout::vertical([Length(topbar_height), Min(1)]).areas(area);
let [sidebar_area, main_area] =
    Layout::horizontal([Length(sidebar_w), Min(1)]).areas(body_area);
```

The sidebar is therefore cut from a frame the topbar has already shortened, and every topbar row
pushes the sidebar down. The intended relationship is the reverse: the sidebar anchors the left
edge for the full height and the topbar continues from it.

The shipped spec did not detect this. Its topbar requirement constrains vocabulary and reserves
geometry, but states no relationship between the two regions. The implementation satisfies the
spec exactly, and strict validation passed, because the spec was self-consistent and incomplete.

The same omission produced the dock. A token-driven panel on the right edge was intended. What
exists is `DockConfig { enabled, side, size }` hosting a live terminal runtime with focus routing
and hidden backing tabs. A terminal host and a chrome panel share an edge and nothing else.

## Goals / Non-Goals

**Goals**

- Make the render relationship explicit in the spec, so the omission cannot recur.
- Anchor the sidebar to the full-height left edge and move the topbar beside it.
- Add a right chrome panel that reuses the Agent-sidebar renderer and token vocabulary.
- Separate "dock" from "chrome panel" in the spec text.

**Non-Goals**

- Changing the token vocabulary. The data layer already matches intent.
- Changing dock behaviour. The dock is retained as-is; only the spec's description of it changes.
- Migrating configuration. Existing `[ui.topbar] rows` values stay valid.
- Adding a second renderer. The right panel reuses the sidebar path or it is not built.

## Decisions

### 1. Invert the root split order rather than special-case the topbar

Take the side columns from the full frame first, then split the remainder vertically:

```rust
let [sidebar_area, rest]     = Layout::horizontal(...).areas(area);
let [topbar_area, main_area] = Layout::vertical(...).areas(rest);
```

This produces the intended geometry as a consequence of ordering, with no conditional logic. The
alternative — keeping the vertical split and offsetting `topbar_rect.x` by `sidebar_w` — yields the
same pixels but leaves the layout tree stating the wrong relationship, which is the defect being
corrected. The clamp preserving one main-content row is retained unchanged.

### 2. Reuse the sidebar renderer for the right panel, or do not ship it

`src/ui/sidebar.rs` already exposes the reuse surface at crate visibility:
`resolved_token_spans`, `agent_panel_entries_from`, and `agent_panel_status_key`. The topbar
consumes exactly these. The right panel consumes the same functions.

A copied resolver would reintroduce the split this change exists to close: two vocabularies
drifting apart, with the spec unable to say which is authoritative. If reuse proves impossible, the
correct outcome is to stop and re-scope, not to fork the resolver.

### 3. Model the right panel on `SpacesSidebarConfig`, not on `DockConfig`

`src/config/sidebar.rs` already carries `AgentsSidebarConfig` and `SpacesSidebarConfig`, both
holding typed row vectors and `row_gap`. The right panel takes the same shape plus `enabled` and
`width`. Its rows are `AgentSidebarToken` rows, identical to `topbar_rows`.

`DockConfig` is the wrong model. Its `side` and `size` describe an edge-mounted terminal, and
copying it would repeat the conflation this change corrects.

### 4. Retain the dock and state the distinction in the spec

The dock is not replaced. The maintainer asked for the right panel "as well". Both may occupy the
right edge, so the spec gains a requirement covering that overlap and the shared clamp.

The added requirement also states what a chrome panel is not: it allocates no terminal runtime and
takes no part in focus routing or backing-tab state. That sentence is the durable guard against a
third implementation drifting the same way.

### 5. Express the relationship as a requirement, not as design prose

The modified requirement carries the sentence the original lacked: the sidebar owns the full-height
left edge and the topbar occupies the main-content width beside it. Two scenarios assert it —
one for the resolved case, one proving an unresolved topbar still leaves the sidebar full height.

A design document alone would not have prevented this. `--strict` checks requirements and
scenarios, so the constraint has to live there to be machine-checked.

## Risks / Trade-offs

- **Visible movement for existing users.** Anyone with `[ui.topbar]` configured sees the topbar
  shift right by the sidebar width. No configuration breaks and no token is removed. This is the
  intended correction, not a regression.
- **Narrow terminals lose width faster.** Two side panels plus a right dock can compete for the
  same edge. Mitigated by extending the existing clamp: at least one main-content column survives,
  and the proposal requires a test at a narrow width with both panels enabled.
- **Reuse surface is `pub(crate)`.** The right panel lives in the same crate, so no visibility
  change is expected. If one proves necessary, that is a signal the reuse boundary is wrong and
  warrants re-scoping rather than widening visibility.
- **Full-height sidebar changes scroll geometry.** `normalized_workspace_scroll` derives from the
  sidebar rectangle, which grows by the topbar height. Row counts shift by one or two. The geometry
  tests in task 1 cover this.

## Migration Plan

No data or configuration migration. The change is layout and additive configuration.

1. Land the layout inversion with geometry tests. This alone corrects the reported defect.
2. Add the right-panel configuration, defaulting to disabled so no existing install changes.
3. Render the right panel through the shared sidebar path.
4. Apply the spec changes and strict-validate.

Steps 2 and 3 are independently revertible. Step 1 stands on its own if the panel is deferred.

## Open Questions

None. Both decisions the exploration could not settle — the render model and the dock disposition —
were answered by the maintainer during the exploration and are recorded in `proposal.md`
`## Decisions` as `decided-by: leo`.

## Apply baseline

- Base: shepherd@cfb8358d
- Paths confirmed at that revision: `src/ui.rs`, `src/ui/topbar.rs`, `src/ui/sidebar.rs`,
  `src/ui/sidebar/tokens.rs`, `src/config/sidebar.rs`, `src/config/dock.rs`, `src/app/state.rs`,
  `openspec/specs/configurable-chrome/spec.md`
