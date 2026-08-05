# session-provider-status

## Why

The topbar renders provider, usage, and session-context values that shepherd does not own. An
outside process computes them, caches them on disk, and pushes them in through metadata reporting
on its own schedule. Shepherd is a passive sink: no store, no demand signal, no cadence, and no
way to tell an absent value from a broken producer.

That gap is not theoretical. On 2026-08-05, three of four producers were down —
`context-floor` and `local-account-signal` failing in microseconds, `llmtrim` killed at 2.24s —
and the topbar rendered a bare workspace name. Nothing in shepherd could report that anything was
wrong, because from shepherd's side an unreported token and a dead producer are the same thing.

The sidebar already solved this. `space-project-status` owns its domain type, its store, its
scheduling, and its business layer; it invokes providers itself, on demand, off the render path,
and every failure degrades to an absent token. This change gives session/provider facts the same
seven layers, described in `CONTEXT.md`.

## What Changes

- Add a domain type for the session/provider fact and cache it on the object that owns it, inside
  `AppState`, alongside the existing `cached_project_status` precedent.
- Add a business layer under `src/app/` that resolves per-token demand from the configured rows,
  dedupes work, invokes provider adapters, parses results, and writes the store — off the render
  path, on its own cadence.
- Add one provider adapter to prove the layering end to end, returning `Option` and degrading to
  no value on every failure class.
- Expose the values as first-class typed tokens through a pure accessor, so a configured row
  resolves them the way `proposals` and `beads` already resolve.
- Shepherd pulls. No shepherd surface depends on an outside process pushing values in.

The remaining provider adapters (llmtrim, anthropic, claude, codex, pi, openai) are deliberately
out of scope here and are structured by this change's final task, not implemented by it.

## Capabilities

### New Capabilities

- `session-provider-status` — shepherd-owned session and provider telemetry: its domain type,
  store, demand-driven refresh, adapter failure semantics, and token exposure.

## Impact

- `src/workspace/project_status.rs`, `src/app/project_status_refresh.rs` — read as the reference
  implementation; not modified.
- `src/config/topbar.rs`, `src/config/sidebar.rs` — token vocabulary gains the new tokens.
- `src/ui/topbar.rs` — resolves the new tokens through the accessor.
- `src/app/state.rs` — the store field and its scheduling counterpart.
- `CONTEXT.md` — the layering this change instantiates.
- Operators currently receiving these values through pushed metadata keep working: the `$custom`
  metadata path is untouched, and the new tokens are additive.

## Preconditions

- premise: the seven-layer pattern exists and is proven — verified: `ProjectStatusSnapshot` +
  `Workspace.cached_project_status` + `src/app/project_status_refresh.rs` +
  `Workspace::project_status()` @ shepherd@b5fc4abe
- premise: provider failure already has a defined degradation contract to copy — verified:
  `run_provider` returns `None` for "binary absent, spawn error, non-zero exit, timeout",
  `src/app/project_status_refresh.rs:194` @ shepherd@b5fc4abe
- premise: demand-driven refresh is already specified — verified: `openspec/specs/space-project-status/spec.md`
  Requirement "Project status refresh is demand-driven" @ shepherd@b5fc4abe
- premise: the current push path is failing in production, not hypothetically — verified:
  `shepherd-state doctor` reported 3 of 4 chrome sources unavailable and pane `w4:p1` carried
  `tokens: (none)` with a live agent @ 2026-08-05
- `test -f src/app/project_status_refresh.rs` → exits 0
- `test -f CONTEXT.md` → exits 0

## Decisions

- Control direction — chosen: shepherd pulls on its own demand-driven cadence; rejected: keeping the
  outside-process push, because it leaves shepherd unable to distinguish an absent value from a
  broken producer; decided-by: leo
- Adapter mechanism — chosen: left free per adapter, since spawning a command and reading a file are
  both valid ways to satisfy the `Option` contract; rejected: one global shell-out-vs-native rule,
  because it forces unrelated sources into one decision; decided-by: leo
- Scope of this change — chosen: the layer plus exactly one adapter, with the remaining adapters
  structured by a task here; rejected: specifying all adapters at once, because the layering should
  be proven before it is multiplied across five providers; decided-by: leo
- Store location — chosen: a cached field on the domain object inside `AppState`; rejected: a disk
  store or a plugin-owned snapshot, because render purity and PTY-free testability both depend on
  the value being in `AppState`; decided-by: default

## Done Means

- A configured row renders provider and session-context values that shepherd itself resolved.
- With every provider unavailable, the tokens elide exactly as an unconfigured token does, and no
  error dialog or user-visible warning appears.
- No configured consumer of a token means no provider work is attempted at all.
- A slow provider never blocks a frame; the previously resolved value renders until the refresh
  completes.
- Operators relying on the existing pushed-metadata tokens see no change in behaviour.

## Testing

- A demand test asserting that a configuration with no consuming row performs no provider work.
- A degradation test per failure class — absent binary, non-zero exit, timeout, unparseable body —
  asserting the token resolves to no value and no other token is affected.
- A render-path test asserting a refresh in flight does not block a frame and the cached value
  renders meanwhile.
- A cadence test asserting this refresh runs independently of the Git status interval.
- An accessor test asserting the token resolves from the store rather than recomputing.
- `just check` passes.
