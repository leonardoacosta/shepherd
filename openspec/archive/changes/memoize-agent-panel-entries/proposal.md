# Memoize agent_panel_entries into ViewState

Base commit: `1de05dc2` · Route: proposal · Effort: S · Confidence: HIGH · Category: performance

## Why

Resolves advisory finding PERF-05 (audit against `1de05dc2`).

`agent_panel_entries(app)` (`src/ui/sidebar.rs:112-180`) builds a fresh
`Vec<AgentPanelEntry>` where each entry carries five `String`/`Option<String>`
fields **plus two `HashMap<String, String>`** (`state_labels`, `tokens` —
`src/ui/sidebar.rs:23-40`). It is rebuilt four to six times per frame:

- `src/ui.rs:240` — `compute_view_internal` calls `agent_panel_scroll_metrics`,
  which calls `agent_panel_bottom_start` (`sidebar.rs:604`) **and**
  `agent_panel_visible_count_from` (`sidebar.rs:589`) — two rebuilds.
- `sidebar.rs:846` rebuilds again during render.
- `agent_panel_scrollbar_rect` (`sidebar.rs:654`) triggers more.

At up to ~62 fps, the entire agent-panel model — including two HashMap
allocations per agent — is constructed and thrown away several times per frame,
and the cost scales with pane count (worst exactly when the multiplexer is
busiest). The entries are a pure function of `AppState`, and AGENTS.md already
forbids mutating state during render, so a computed-once cache is safe.

## What changes

Compute `agent_panel_entries` once per frame in `compute_view_internal`, store it
on `ViewState`, and have the render and hit-testing call sites read the cached
value instead of recomputing.

## Acceptance

- `just check` passes.
- Existing agent-panel tests green — several already call `agent_panel_entries`
  directly (`src/app/actions.rs:4303`, `src/app/agent_view.rs:471+`,
  `src/ui/mobile.rs:1303`), so behavior is pinned.
- No visible change to the sidebar/agent panel.

## Out of scope

- Non-render callers that legitimately need entries outside a frame
  (`src/app/actions.rs:1508,1530`, `src/app/input/navigate.rs:721,727`,
  `src/app/input/sidebar.rs:356,505`) — they may either share the cache when a
  fresh `ViewState` is available or keep calling the function; do not force the
  cache onto call paths that run without a computed view.
- The mobile path's `all_agent_panel_entries` (`src/ui/mobile.rs:990`) is a
  distinct function — decide separately whether it warrants its own cache; not
  required here.
