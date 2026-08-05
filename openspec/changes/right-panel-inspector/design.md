## Context

`anchor-chrome-to-sidebar` added the right chrome panel with a single constraint, stated in its
Decision 2: "the right panel reuses the sidebar path or it is not built." That held. The panel
renders `AgentSidebarToken` rows through `sidebar::resolved_token_spans`, the same call the topbar
makes, with no forked resolver.

What it did not settle is what the right edge is *for*. The panel inherited the topbar's token
vocabulary because that was the reuse surface available, and the topbar's vocabulary describes the
focused pane. So the right panel currently answers the same question the topbar answers, on the
opposite edge, at full height.

The question nothing answers is the state of the work: open proposals, tracked issues, the run in
flight, the diff on the branch. Those are five different fact sources with five different
refresh characteristics, and they do not fit one row of tokens.

The panel is unreleased — absent from `CHANGELOG.md` and from `src/config/sidebar.rs` at
`preview-2026-07-21-0f10e1453a7f`. This is the last change that can restructure its configuration
without a migration.

## Goals / Non-Goals

**Goals**

- Establish the tab surface: selection, scrolling, width-resolved affordance, pointer hit-testing.
- Restructure configuration so each tab owns its own token rows.
- Ship one tab — proposals — end to end, over the provider that already exists.
- Amend `configurable-chrome` so the chrome-panel definition forbids terminal semantics rather
  than all interaction.

**Non-Goals**

- The tracked-issues, run-status, git-diff, and host-timer tabs. Each is its own change.
- Keyboard focus routing into the panel. Pointer only.
- Changing the Agent or Space token vocabularies. Both are correct and untouched.
- Changing dock behaviour, pane focus, or terminal input routing.
- A second row renderer. Same constraint as `anchor-chrome-to-sidebar` Decision 2: reuse the
  shared path or do not ship the tab.

## Decisions

### 1. Each tab is a configuration sub-table owning its own rows

```toml
[ui.right_panel]
enabled = true
width = 36
tabs = ["props"]

[ui.right_panel.props]
rows = [["proposal_name", { token = "proposal_tasks", dim = true }]]
```

The panel keeps only what is panel-wide. `tabs` declares order, enablement, and — as its first
entry — the start tab.

The alternative was a panel-level `mode = "tokens" | "inspector"` switch retaining the old rows
beside the new surface. That gives the panel two personalities and two render paths permanently,
to preserve a configuration nothing has shipped against. The second alternative — a third
independent chrome region beside the token panel and the dock — puts three occupants on one edge
competing for the same columns.

`tabs` is an explicit array rather than implicit sub-table order because TOML table order does not
survive deserialization, which would leave the start tab undefined.

### 2. One renderer, one token vocabulary per tab

The inspector is not five tab renderers. It is the shared row renderer plus a token vocabulary
per tab, each elided and styled by the rules already in `resolved_token_spans`. A bespoke renderer
per tab reintroduces the resolver fork `anchor-chrome-to-sidebar` Decision 2 exists to prevent,
and the drift would be worse here than it was for the topbar, because five vocabularies would
diverge instead of two.

Vocabularies are separate enums per tab, mirroring the existing split between
`AgentsSidebarConfig` and `SpacesSidebarConfig`. A token used in the wrong tab is then a named
configuration error rather than a silent elision that reads as missing data.

### 3. Pointer interaction, not focus

The left sidebar accepts clicks and scroll, allocates no terminal runtime, and never receives
keyboard input. That is the precedent, and it is what the amended requirement should say.

The shipped requirement says a chrome panel "accepts no focus" and "takes no part in focus routing
or backing-tab state." What that sentence was protecting is that a chrome panel is not a second
dock — it must not host a terminal or sit in the terminal input path. It over-stated the
constraint by ruling out interaction entirely.

The amendment narrows "focus routing" to "terminal input routing" and states the pointer
allowance explicitly, on the same terms as the left sidebar. Keyboard focus routing into the panel
was rejected: it is the larger amendment, and it puts a second consumer in front of the terminal
input path for no gain the pointer does not already deliver.

### 4. Width resolves the tab affordance, and the default width cannot fit a tab bar

Content width is `panel width - 2`: one divider column against main content, one left pad. At the
default width of 24 that is 21 columns. The five eventual tab labels joined by single separators
measure 28 columns, and 36 with spaced separators.

So the default install cannot render a tab bar at all. Three tiers:

| Content columns | Tab affordance |
| --- | --- |
| < 28 | Cycling header naming the selected tab and its position |
| 28–35 | Tight tab bar, selected tab underlined |
| ≥ 36 | Spaced tab bar |

Narrow, at 21 content columns:

```
│ ‹ props ›       1/1
│ ─────────────────────
│ 4 open · 2 in prog.
│ ▸ right-panel-insp…
│ ▸ windows-test-cov…
```

Default tier, at 33 content columns:

```
│ props
│ ▔▔▔▔▔────────────────────────────
│ 4 open · 2 in progress
│ ▸ right-panel-inspector    6/14
│ ▸ windows-test-coverage     0/5
```

Clipping tab labels was rejected: at the default width it renders a truncated first label with no
indication that other tabs exist, which is worse than naming the selected tab and its position.

This tiering mirrors the responsive tiers the panel's visual reference already uses, and it is
measured from the label set rather than chosen.

### 5. Geometry follows configured tabs, not resolved rows

Today `right_panel_w` is gated on `resolved_right_panel_rows(app).is_empty()` — the panel reserves
width only when its rows resolve a value. An inspector cannot work that way: a tab whose provider
has not returned yet, or whose checkout has no proposals, still owns its width. The gate moves to
"the panel is enabled and declares at least one tab."

The existing clamp is retained unchanged: both side panels are bounded so at least one
main-content column survives.

### 6. The proposals domain type widens; the refresh loop does not change

`ProposalCounts { open, in_progress }` becomes a type carrying the same two derived counts plus
the per-proposal items the tab lists. `parse_proposal_counts` already reads `name`,
`completedTasks`, and `totalTasks` per change and discards the names; the tab needs them kept.

Demand resolution, per-checkout deduplication, the independent refresh cadence, running off the
render path, and per-provider fail-open are unchanged. The demand source moves from "a configured
space row consumes the `proposals` token" to "that, or a configured right-panel tab is the
proposals tab" — one predicate gains a second disjunct.

This keeps the change inside the seven-layer model `space-project-status` established: the store
stays on the domain object in `AppState`, scheduling stays on the runtime `App`, the business
layer stays off the render path, and render stays a pure read.

### 7. Selected tab and scroll are client presentation state

Per the runtime/client boundary guardrail: the proposal inventory is a shared runtime fact and
stays where it already lives. The selected tab, the scroll offset, and the resolved tab rows are
TUI presentation and live only in the client layer. No new API field, event, or socket message is
added by this change.

## Risks / Trade-offs

- **The default width is the worst case.** Every tab has to be legible at 21 content columns,
  which is the width the tier table and the narrow sketch above are written against. Mitigated by
  making the narrow tier a first-class rendering path with its own tests, not a degraded fallback.
- **Removing panel-level `rows` is a breaking configuration change** for anyone running the
  unreleased build with `[ui.right_panel] rows` set. Nothing released depends on it, and the
  topbar renders the identical vocabulary. This was asked and answered rather than assumed.
- **Four tabs are being designed for and not built.** The surface could be shaped wrongly for
  tabs that do not exist yet. Mitigated by shipping the tab whose provider already exists, so the
  shape is proved against a real fact source rather than an imagined one, and by keeping the
  per-tab vocabulary boundary so a later tab adds an enum and a provider rather than editing the
  surface.
- **Overlaps the in-flight `session-provider-status` proposal** on three files:
  `src/config/sidebar.rs`, `src/ui/topbar.rs`, and `src/app/state.rs`. The sharpest of the three is
  `src/ui/topbar.rs` — that proposal adds token resolution to it while this one removes
  `resolved_right_panel_rows` and `render_right_panel` from it. Neither change contradicts the
  other, but they must not be applied concurrently in the same checkout. `src/workspace/
  project_status.rs` is not a conflict: that proposal reads it as a reference implementation and
  does not modify it.
- **Hit-testing on a surface whose rows come from a provider.** A test that seeds only the panel
  rectangle fakes a computed frame and measures a different list than the render produced. The
  tests seed the computed frame entries, matching how the sidebar's own hit-test tests work.

## Migration Plan

No data migration. Configuration restructuring on an unreleased surface.

1. Restructure configuration and geometry, with the panel rendering an empty selected tab. The
   old panel-level `rows` path is removed in this step.
2. Add the tab surface: affordance tiers, selection, scroll, pointer hit-testing.
3. Widen the proposals domain type and extend demand resolution to the tab.
4. Render the proposals tab through the shared row renderer.
5. Apply the spec changes and strict-validate.
6. Document on the unreleased path.

Steps 1 and 2 stand without step 3. Step 3 is independently useful to the space tokens that
already consume the same provider.

## Open Questions

None blocking. Two deferred to the changes that need them:

- Whether the selected tab persists across restarts, and whether per space or per client. This
  change makes `tabs[0]` the start tab, which is sufficient until a second tab exists.
- Whether a wide mode is needed for a tab whose content does not fit any configured width. That
  question belongs to the git-diff tab, which is the only candidate.

## Apply baseline

- Base: shepherd@b5fc4abe
- Paths confirmed at that revision: `src/ui.rs`, `src/ui/topbar.rs`, `src/ui/sidebar.rs`,
  `src/ui/sidebar/tokens.rs`, `src/config/sidebar.rs`, `src/config/model.rs`, `src/app/state.rs`,
  `src/app/input/mouse.rs`, `src/app/input/sidebar.rs`, `src/workspace/project_status.rs`,
  `src/app/project_status_refresh.rs`, `openspec/specs/configurable-chrome/spec.md`,
  `docs/next/website/src/content/docs/configuration.mdx`, `docs/next/CHANGELOG.md`
