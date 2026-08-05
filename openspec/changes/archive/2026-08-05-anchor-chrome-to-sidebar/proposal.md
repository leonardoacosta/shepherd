## Why

`configurable-chrome` shipped the topbar with the correct data model and the wrong render model,
and the maintainer's intent for a second chrome panel was implemented as a docked terminal.

The topbar already extends the Agent sidebar where data is concerned. `src/ui/topbar.rs` calls
`sidebar::tokens::agent_rows_from` and `sidebar::resolved_token_spans`, and `AppState.topbar_rows`
is typed `Vec<Vec<AgentSidebarToken>>`. That half is correct and this change does not touch it.

Render is not an extension. `src/ui.rs` takes the topbar off the whole frame before the sidebar
exists:

```rust
let [topbar_area, body_area] =
    Layout::vertical([Length(topbar_height), Min(1)]).areas(area);
let [sidebar_area, main_area] =
    Layout::horizontal([Length(sidebar_w), Min(1)]).areas(body_area);
```

The topbar is therefore the sidebar's parent-level sibling and pushes the sidebar down one row.
The intent was the opposite: the sidebar anchors the left edge for the full height, and the topbar
continues from it across the main content only.

The current spec did not catch this. Its topbar requirement constrains the token vocabulary
("using the documented Agent-sidebar token vocabulary and styling rules") and reserves geometry
("reserves two rows of desktop geometry"), but never states the render relationship between the
topbar and the sidebar. The implementation satisfies the spec exactly. The spec was incomplete.

The same gap produced the dock. A chrome panel on the right edge was intended — a token-driven
panel like `[ui.sidebar.spaces]`. What exists is `DockConfig { enabled, side, size }` hosting a
live terminal runtime with focus routing and backing tabs. A terminal host and a chrome panel are
different things, and the spec presents the dock as though it satisfied the panel intent.

## What Changes

- Re-anchor the topbar so the sidebar owns the full-height left edge and the topbar spans the
  main-content width only.
- Add a right chrome sidebar that reuses the Agent-sidebar renderer and token vocabulary, with the
  same elision and styling rules as the left sidebar and the topbar.
- State the render relationship in the spec so chrome regions are defined as sidebar extensions,
  not as independent regions that happen to share tokens.
- Separate "dock" from "chrome panel" in the spec text. The dock keeps its current behaviour and
  scope; the spec stops implying it serves the panel intent.

The dock is retained, not replaced. The maintainer asked for a right sidebar "as well".

## Capabilities

### New Capabilities

None. This change extends `configurable-chrome`.

### Modified Capabilities

- `configurable-chrome` — adds the chrome anchoring relationship, adds the right sidebar, and
  disambiguates the dock from chrome panels.

## Impact

- `src/ui.rs` — split order at the root layout; right sidebar area allocation.
- `src/ui/topbar.rs` — no token changes; area now excludes the sidebar column.
- `src/ui/sidebar/` — panel rendering reused for the right edge.
- `src/config/sidebar.rs` — configuration for the right panel.
- `src/app/state.rs` — right panel rows, typed `Vec<Vec<AgentSidebarToken>>` like `topbar_rows`.
- `openspec/specs/configurable-chrome/spec.md` — modified requirements.
- Users with `[ui.topbar]` configured see the topbar move right by the sidebar width. No
  configuration file changes are required and no token is removed.

## Preconditions

- premise: the topbar already shares the Agent-sidebar token layer — verified: `src/ui/topbar.rs`
  imports `sidebar::tokens::agent_rows_from` and `sidebar::resolved_token_spans`, and
  `src/app/state.rs:1779` types `topbar_rows` as `Vec<Vec<AgentSidebarToken>>`
  @ shepherd@cfb8358d
- premise: the root layout removes the topbar before the sidebar split — verified:
  `src/ui.rs:242-251` @ shepherd@cfb8358d
- premise: the dock hosts a terminal runtime rather than rendering tokens — verified:
  `src/config/dock.rs` `DockConfig { enabled, side, size }` and the `configurable-chrome`
  requirement "Dock geometry follows live workspace occupancy" @ shepherd@cfb8358d
- premise: the current spec does not state a topbar/sidebar render relationship — verified: read of
  `openspec/specs/configurable-chrome/spec.md` requirement "Topbar renders configured rows and
  Agent tokens" @ shepherd@cfb8358d
- `cargo build --release` succeeds before the first edit.

## Decisions

- Chrome render model — chosen: the sidebar anchors the full-height left edge and the topbar spans
  main-content width only; rejected: rendering the topbar inside the sidebar column, because the
  column is too narrow for a five-token row and would elide most configured tokens; decided-by: leo
- Dock disposition — chosen: retain the dock unchanged and add the right sidebar beside it;
  rejected: replacing the dock with the right sidebar, because the maintainer asked for the panel
  "as well"; decided-by: leo
- Scope of the token layer — chosen: leave the token vocabulary untouched; rejected: reworking
  tokens alongside geometry, because the data half already matches intent; decided-by: default
- Existing configuration — chosen: keep `[ui.topbar] rows` valid with no migration; rejected: a new
  config shape, because only the render region changes; decided-by: default

## Done Means

- The left sidebar renders from the first terminal row to the last, and the topbar no longer
  displaces it.
- The topbar begins at the right edge of the sidebar column and spans the remaining width.
- An operator can configure a right chrome sidebar with the same tokens accepted by
  `[ui.sidebar.spaces]`, and it renders on the right edge.
- The right sidebar elides absent tokens and omits an empty row, matching left-sidebar behaviour.
- The spec states that chrome regions are sidebar extensions, and describes the dock as a terminal
  host that is not a chrome panel.

## Testing

- Geometry tests asserting the sidebar rectangle spans full height and the topbar rectangle starts
  at `sidebar_w`.
- A test asserting the topbar rectangle is empty when no row resolves, and the sidebar still spans
  full height.
- Right-sidebar render tests covering token resolution, elision of absent values, and omission of a
  fully unresolved row.
- A clamp test proving at least one main-content column survives when both sidebars are enabled.
- `cargo test` and `just check` pass.
