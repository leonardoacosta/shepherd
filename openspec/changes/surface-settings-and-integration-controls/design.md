## Context

Settings currently requests a 76-column popup, renders six padded section labels through Ratatui `Tabs`, and computes mouse targets independently. At 64 columns the final sections can be clipped even though keyboard cycling reaches them. Integration content requests one row per target but the popup is clamped by terminal height; neither target rows nor the six-line result buffer scroll, and the only TUI action is bulk recommended installation. The backend already exposes typed per-target install/uninstall operations and richer status data.

`AppState` already applies common UI configuration values live and `src/app/config_io.rs` performs atomic comment-preserving single-key edits. Those values are not represented as Settings rows. Agent sort is the exception: a contextual mouse-only sidebar toggle persists the shared key, but its label does not identify itself as a sort control.

Terminal state owns `HookAuthority`, including report source and whether full-lifecycle authority suppresses screen detection. Public `PaneInfo`/`AgentInfo` expose only `screen_detection_skipped`, while session identity/source is separate and can be persisted. Installation state therefore cannot truthfully answer whether an integration currently supplies lifecycle data.

Finally, the configuration reference contains 153 keys across structured and platform-sensitive shapes. An exhaustive TUI form or raw editor widget is disproportionate. The server owns the resolved config path, and Shepherd already knows how to launch a user's editor in an auxiliary PTY for scrollback; unlike scrollback, the real config file must never be treated as a temporary cleanup target.

## Goals / Non-Goals

**Goals:**

- Make Settings section and content navigation correct at 40x20, 64x20, and 80x24 through one pure geometry model.
- Give each integration target reachable status, actions, results, and truthful runtime observations.
- Expose the current state evidence source as a neutral server/API fact with one projection implementation.
- Surface the selected stable visual/interaction preferences through safe existing config persistence.
- Make every remaining configuration key reachable through a server-local editor process with validated reload.

**Non-Goals:**

- A keybinding editor, arbitrary token composer, theme-color editor, exhaustive config form, or embedded TOML editor.
- Treating installation, report recency, or a persisted session reference as health.
- Making Settings layout a server fact or naming protocol fields after rows, cards, sidebars, or badges.
- Changing integration asset behavior, adding dependencies by default, or documenting unreleased behavior in stable docs.

## Decisions

### Use one pure Settings view model for render and input

Introduce a pure computed `SettingsView` (or equivalently named structure) from `&AppState` and the available `Rect`. It contains visible section items and hit rectangles, content/scrollbar rectangles, visible typed rows, action rectangles, and overflow controls. `compute_view()` may normalize selected identities and offsets; `render()` consumes the resulting state without mutation. Keyboard and mouse both resolve section/row actions through the same model.

At widths where all section labels fit, keep the familiar order: Theme, Sound, Toast, Display, Behavior, Integrations, Experiments. When they do not, show a viewport with previous/next chevrons; if even that cannot retain a useful label, show the selected section plus an overflow indicator. Badges travel with their section identity. At less than 40x20, render a safe compact shell with close/navigation affordances but make 40x20 the full reachability contract.

Selection is stored by stable row identity, not display index. Each section owns a scroll offset; normalization keeps the selected selectable row visible after resize, section change, target/status refresh, or dynamic row insertion/removal. Shared scrollbar geometry handles long content.

Rejected: more fixed-width tabs, clipping, or separate mouse coordinate arithmetic.

### Make integration rows target-keyed and operations neutral

Extend the recommendation/view projection to retain target support/availability, installed state, installed path, current version, expected version, and a reason when an operation is unavailable. `IntegrationTarget` is the stable row identity. Unsupported or platform-unavailable targets remain visible with an explanation and disabled actions.

Enter/click opens the selected target action choices. Install is shown when absent, update when outdated, uninstall when installed/current, and reinstall can be omitted in the first version. Uninstall requires the existing modal language and explicit confirmation naming the target/path. Only the selected target is mutated. Bulk recommended install remains a secondary action and never includes unsupported/unavailable targets.

Integration mutations are globally single-flight within the server session so target config edits cannot race. While an operation or bulk sequence is active, mutation actions are disabled but every target and prior detail remains navigable. Bulk work processes eligible targets in registry order and records each result before continuing. Completion refreshes all statuses. Results are stored as target-keyed operation records with success/failure and complete message lines. Settings shows a bounded summary plus a scrollable detail view, so no result or late failure is discarded.

Rejected: concurrent integration mutations, TUI-specific socket methods, implicit uninstall, fixed six-line output, or index-keyed selection.

### Project state authority once at the terminal boundary

Add shared API types:

- `AgentStateSource::Screen`
- `AgentStateSource::Reported { source: String, authority: AgentReportAuthority }`
- `AgentReportAuthority::{ExclusiveLifecycle, Mixed}`

`PaneInfo.state_source` and `AgentInfo.state_source` are optional because panes without a detected/reported agent have no state source. Both are populated by one terminal projection helper. A current active hook authority produces `Reported`; full-lifecycle authority maps to `ExclusiveLifecycle`, other accepted reports map to `Mixed`, and screen-manifest state maps to `Screen`. Expired/cleared authority falls back to screen when screen evidence exists or to absent when no current agent state exists.

Keep `screen_detection_skipped` for compatibility and derive it from `ExclusiveLifecycle`. Do not expose `reported_at` as health or freshness. Existing `agent_session` remains the separate session-identity fact and can be displayed only as a count/source category, never by identifier.

Integration observations aggregate current pane projections by integration source: state-reporting pane count, exclusive/mixed distinction when useful, and session-identity pane count. Only an exact canonical source id owned by a registered `IntegrationTarget` contributes to that target; unknown/custom sources are not guessed or misattributed. Zero observation is copy such as `installed; not currently observed`, never an error verdict.

Rejected: using install versions, the skip boolean, or report age as green/yellow/red health.

### Add curated Display and Behavior rows

Display gains pane borders, pane gaps, hide-single-tab-bar, and Agent sort alongside existing chrome controls. Behavior owns close confirmation, new-tab naming prompt, new-workspace naming prompt, copy-on-select, and mouse scroll lines. Its terminal `edit config.toml` action opens the advanced server operation described below; it is not a persisted preference. Values use typed row identifiers and explicit bounds from `Config`; numeric scroll speed changes through bounded increment/decrement controls.

Each action calls a dedicated one-key `config_io` writer and then the existing live reload path. If a value cannot be applied live at implementation time, its row says `next launch` and the task must test that boundary; it must not pretend to have taken effect. Malformed existing TOML follows current safe fallback behavior and yields bounded actionable feedback.

The sidebar header becomes `sort: grouped` or `sort: priority`, gains keyboard reachability through Settings, and both paths write only `ui.agent_panel_sort`. No second state field is added.

Rejected: startup-sensitive/mouse-capture/host-cursor/right-click controls, keybindings, themes, experimental values, or arbitrary row composition in native Settings.

### Represent advanced editing as a server operation and ordinary pane runtime

Add neutral JSON method `server.config.edit` with no client-local path. The server resolves its active config path, constructs platform argv, and starts a server-owned auxiliary pane. The response is `ConfigEditStarted { pane: PaneInfo, already_open: bool }`. A requesting TUI may present that pane as an overlay; API semantics do not mention overlays or Settings. The auxiliary pane is excluded from disk restore and ordinary tab navigation, follows existing live runtime cleanup, and is discoverable by its returned pane identity while active.

Only one config editor operation may exist per server session. A duplicate call returns the existing pane with `already_open: true`. If no editor can be launched, return `config_editor_unavailable` without creating or modifying the file. If the current architecture has no session surface capable of hosting the auxiliary pane, return `config_editor_surface_unavailable`; the method does not accept a workspace/tab id and does not manufacture unrelated session identity. A client disconnect does not kill a successfully started server-owned edit.

Linux/macOS preserve `$VISUAL`/`$EDITOR` command semantics while passing the resolved path as a positional argument rather than interpolated shell text; fallback is `vi`. Windows parses the editor command into argv using the existing safe logic and falls back to Notepad. The config path is never registered in the overlay `temp_files` cleanup list.

Before launch, record whether the file exists and its content digest without changing it. On process exit, inspect the final server-side bytes even when the editor returned nonzero. Unchanged bytes (including an initially absent file that remains absent) produce `unchanged`, or `editor_failed` when the process itself failed. Changed valid TOML runs the same reload pipeline as `server.reload_config` and produces `reloaded`; a nonzero editor status is retained as a diagnostic rather than discarding a valid saved edit. If a previously existing file was deliberately removed, normal reload activates defaults and reports `reloaded` with a removal diagnostic. Invalid TOML or an unreadable changed file remains untouched, the last valid runtime configuration stays active, and diagnostics include path plus line/column when available. External edits are not locked; the final bytes at process exit are authoritative.

Emit `server.config_edit_finished` with pane id, outcome (`reloaded`, `unchanged`, `invalid`, or `editor_failed`), and diagnostics; add the corresponding subscription/event kind. The TUI retains the outcome in bounded Settings feedback and can reopen the same file immediately.

This new public method/event and state-source fields are generated into schema and TypeScript types. Compare the source protocol version with the latest release: bump only if the current source is not already ahead, then update fixtures once.

Rejected: a TUI-private socket command, client-provided path, embedded editor, whole-file programmatic rewrite, unconditional reload, or deletion after exit.

## Risks / Trade-offs

- **The combined Settings work spans UI, protocol, config, and runtime identity** → Land in characterization-first stages, run the required roundtable, and use adversarial identity invariants around auxiliary panes and projection.
- **Dynamic rows can invalidate an index** → Store typed identities and normalize through the pure view model after every data/size transition.
- **Partial integration operations can hide the actionable failure** → Preserve complete target-keyed records and verify a late failure is reachable at 40x20.
- **State source can become an accidental health contract** → Omit timestamps/freshness colors and use descriptive counts only.
- **An editor can corrupt the file** → Preserve the user's exact bytes, validate before runtime reload, retain last valid runtime config, and provide reopenable diagnostics.
- **Auxiliary-pane lifecycle can disturb workspace identity** → Exclude it from ordinary navigation/persistence, return stable pane identity, and exercise adversarial `AppState`/`Workspace` invariants.
- **External processes can edit the file concurrently** → Guarantee only Shepherd-initiated single-flight editing; document that external last-writer behavior is unchanged and validate the bytes present at process exit.

## Migration Plan

1. Characterize existing Settings geometry, selection, config writes, integration results, authority projection, and overlay exit behavior; run the broad-change roundtable.
2. Introduce the pure Settings view model and responsive navigation/content scrolling without adding controls.
3. Add the shared state-source API projection and generated consumers, preserving the compatibility boolean.
4. Build the per-target integration manager and descriptive observation aggregation.
5. Add native Display/Behavior rows and unify Agent sort labeling/persistence.
6. Add `server.config.edit`, auxiliary-pane lifecycle, validated exit reload, events, and TUI presentation.
7. Update next docs/generated references and run focused, platform, invariant, and full checks.

Rollback removes optional fields/method/event and UI entry points; one-key config values and user-edited TOML require no data migration. Never roll back by overwriting a user's config.

## Open Questions

None. Exact internal type/module names may follow existing conventions, but the wire names, state-source semantics, scoped preferences, and error behavior above are fixed.
