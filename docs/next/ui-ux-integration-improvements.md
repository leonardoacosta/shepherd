# Shepherd UI, UX, and integration improvements

> Advisory research record captured at explicit user request. This file is not an execution
> ledger, OpenSpec proposal, or replacement for repository issue state. Any selected capability
> must still enter the proposal or ad-hoc lane defined by `AGENTS.md`.

## Research frame

- Audited revision: `062955ae513d4b0e3281e957043b190bdfd90ee6` on clean `dev`.
- Researched: Settings geometry/state/input, integration install and runtime authority, onboarding,
  configuration persistence, sidebar/topbar composition, advanced configuration access, tests,
  unreleased documentation, prior OpenSpec decisions, and relevant git history.
- Not validated live: real agent CLI installation, interactive PTY screenshots, Windows editor
  behavior, physical mobile terminals, or a real integration reporting session.
- Drift rule: recheck cited assumptions if `src/ui/settings.rs`, `src/app/input/settings.rs`,
  `src/config/sidebar.rs`, `src/terminal/state.rs`, or integration target membership changes.
- Verification baseline: the audited head previously passed `just check`; this research pass did
  not rerun tests because it changed documentation only.

## Prior art and ownership

- `openspec/changes/archive/2026-08-04-surface-configurable-chrome/` established Display's pure,
  scrollable row model and deliberately left arbitrary topbar token editing in `config.toml`.
- `src/ui/tabs.rs:42-255` already has pure overflow geometry that keeps the active workspace tab
  visible and provides mouse scroll controls. `src/ui/scrollbar.rs:14-153` provides reusable
  scrollbar geometry and rendering.
- `src/app/config_io.rs:4-175` provides atomic, comment-preserving single-key writes followed by
  live config reload.
- Git history points to Ogulcan/Can as the primary Settings, runtime-authority, and integration
  owners. Leonardo owns the fork's recent topbar/dock and Pi integration work. This is historical
  ownership, not a formal CODEOWNERS declaration.
- The active `spike-remote-agent-catalog` and `windows-test-coverage` OpenSpec changes do not own
  the outcomes below.

## Confirmed findings

### UI-01: Integration Settings clips targets and results

- **Evidence:** `src/api/schema/integrations.rs:33-50` defines 15 targets;
  `src/integration/registry.rs:229-248` produces a recommendation for each supported target;
  `src/ui/settings.rs:24-32` requests one popup row per target, but
  `src/ui/widgets.rs:39-48` clamps the popup to the terminal; and
  `src/ui/settings.rs:314-350` renders one unscrolled paragraph.
- **Impact:** At 80x24, at most roughly seven integration rows fit before output. Later targets are
  invisible. With installation output, the list can shrink further.
- **Additional failure:** `src/ui/settings.rs:247-281` caps result output at six rows, while
  `src/app/mod.rs:1361-1378` may append at least one result for every attempted target. A failure
  after several successes can be hidden.
- **Input gap:** `src/app/input/settings.rs:434-450` has no integration up/down navigation and
  `src/app/input/settings.rs:528-582` maps no integration row for mouse input.
- **Effort / risk / confidence:** M / MED / HIGH.
- **Route:** feature `improve-integration-settings-ux`.

### UI-02: Integration Settings is a passive status board

- **Evidence:** Settings offers only `InstallRecommendedIntegrations`
  (`src/app/input/settings.rs:265-289`). Per-target install and uninstall already exist in
  `src/api/schema/integrations.rs:3-31`, `src/app/api/integrations.rs:6-45`, and
  `src/cli/integration.rs:60-92`.
- **Missed detail:** `IntegrationStatus` knows path, installed version, and expected version at
  `src/integration/types.rs:120-127`; `IntegrationRecommendation` drops the version fields at
  `src/integration/types.rs:137-159`.
- **Impact:** Users must leave the TUI to update, inspect, or uninstall one integration, despite
  the backend already supporting the operation.
- **Effort / risk / confidence:** M / MED / HIGH.
- **Route:** include in feature `improve-integration-settings-ux` after responsive list behavior.

### UI-03: Settings section navigation is not responsive

- **Evidence:** `src/ui/settings.rs:21-22` fixes the desired popup width at 76 and
  `src/ui/settings.rs:69-98` renders all six padded section labels on one line. At a 64-column
  terminal the popup inner width is 58 columns, while the labels need approximately 63 columns
  before an integration-update badge. `src/app/input/settings.rs:501-520` still calculates mouse
  targets beyond the clipped viewport.
- **Impact:** The active Integrations or Experiments section can be invisible and unclickable on
  the mobile threshold even though keyboard cycling still reaches it.
- **Effort / risk / confidence:** M / LOW / HIGH.
- **Route:** prerequisite inside `improve-integration-settings-ux`, not another fixed-width tab.

### UI-04: Everyday UI behavior remains configuration-only

- **Evidence:** `src/app/state.rs:1123-1150` exposes only Theme, Sound, Toast, Display,
  Integrations, and Experiments. `src/config/model.rs:777-818` also contains pane borders/gaps,
  single-tab-bar visibility, close confirmation, naming prompts, copy-on-select, mouse scroll
  speed, sidebar ordering, and other UI behavior. `src/app/mod.rs:1462-1505` applies these values
  live.
- **Impact:** Stable, frequently understood preferences require users to discover and edit TOML.
- **Effort / risk / confidence:** M / LOW / HIGH.
- **Route:** feature `surface-everyday-ui-preferences`, dependent on responsive Settings.

### UX-01: Onboarding can teach the wrong shortcuts

- **Evidence:** `src/ui/onboarding.rs:15` and `src/ui/onboarding.rs:72-95` hardcode `ctrl+b` and `?`.
  Effective prefix/help bindings are configurable, and `src/ui/menus.rs:41-55` plus
  `src/ui/keybind_help.rs:62-75` already render them dynamically.
- **Impact:** First-run instructions are false for customized or provisioned configurations.
- **Effort / risk / confidence:** S / LOW / HIGH.
- **Route:** ad-hoc task `render-effective-onboarding-shortcuts`.

### DOC-01: Hermes integration documentation contradicts the runtime

- **Evidence:** `src/integration/assets/hermes/__init__.py:1-91` reports only resumable session
  identity via `pane.report_agent_session`. `src/detect/mod.rs:283-297` explicitly excludes Hermes
  from full-lifecycle authority and classifies it as session-identity-only. The behavior is pinned
  by `src/detect/mod.rs:774-780` and `src/terminal/state.rs:2316-2342` tests.
- **Contradiction:** `docs/next/website/src/content/docs/integrations.mdx:8,58,215` and
  `docs/next/website/src/content/docs/agents.mdx:21` still describe Hermes as lifecycle-authoritative.
  The next integration guide also says Hermes version 2 at line 65, while
  `src/integration/mod.rs:170` and the bundled asset declare version 4.
- **Impact:** Users are told installation changes state authority when the current runtime
  intentionally leaves Hermes state to its screen manifest; the documented update version is
  also stale.
- **Effort / risk / confidence:** S / LOW / HIGH.
- **Route:** ad-hoc documentation correction across `docs/next` and translations, with config/docs
  parity checks. Stable docs should only change if the corrected runtime has been released there.

## Deferred-item research and decisions

### R-01: Integration health and runtime observation

#### What exists

- Installation state is a filesystem/version fact (`src/integration/types.rs:120-159`). It does
  not prove that an agent process loaded an integration.
- `TerminalState` already owns a live `HookAuthority` containing source, agent, state, session
  reference, and report time (`src/terminal/state.rs:5-25,119-148`).
- `TerminalState::full_lifecycle_hook_authority_active()` is the authoritative test for exclusive
  lifecycle reporting (`src/terminal/state.rs:1737-1761`).
- `AgentInfo.screen_detection_skipped` exposes only the resulting boolean
  (`src/app/agents.rs:360-395`, `src/api/schema/agents.rs:183-223`). The source and whether a report
  is exclusive are not public.
- `AgentInfo.agent_session` and `PaneInfo.agent_session` expose a stored session reference and its
  source (`src/api/schema/agents.rs:225-230`, `src/api/schema/panes.rs:394-428`), but that reference
  can be persisted and is not evidence of a currently live reporter.
- `agent explain` knows when full-lifecycle authority is active, but returns that through an
  explanation-specific JSON object (`src/app/api/agents.rs:154-196`), not a stable runtime field.

#### Options considered

1. **Derive health from installed/current version.** Rejected: it confuses deployment with runtime
   observation.
2. **Treat `screen_detection_skipped` as integration health.** Rejected: it hides the reporting
   source and says nothing useful for session-only integrations.
3. **Use report age as green/yellow/red health.** Rejected for now: lifecycle reporters may remain
   legitimately idle, and `reported_at` is arbitration evidence rather than a heartbeat contract.
4. **Expose neutral runtime source facts and let clients describe them.** Selected direction.

#### Decision

Add a neutral, server-owned state-source projection to both `PaneInfo` and `AgentInfo`. The minimal
honest model should distinguish:

- screen-derived state;
- reported state with a source identifier;
- whether that report is exclusive full-lifecycle authority or may still yield to screen evidence;
- existing session-identity source separately, without calling it live health.

Integration Settings may then aggregate descriptive facts such as `reporting state in 2 panes` or
`session identity available in 1 pane`. It must say `installed; not currently observed` rather
than `broken` when no pane provides runtime evidence. Do not add a single red/green health light.

- **Suggestion:** Expose neutral agent state-source facts and consume them as descriptive
  integration observations.
  - **Reasoning:** The runtime already owns exact hook authority, while public clients receive an
    ambiguous boolean.
  - **Definition of Done:** Mechanical: typed schema field on PaneInfo/AgentInfo, one terminal
    projection helper, generated schema/client/docs updated; behavior: lifecycle and session-only
    integrations receive truthful, distinct labels; done-when: OpenSpec is archived after focused
    terminal/API/UI tests and `just check` pass.
  - **Watch out:** This crosses the server/client contract. At implementation time compare
    `src/protocol/wire.rs::PROTOCOL_VERSION` with the latest release; audited source is 19 and the
    latest inspected release is 17, so do not bump blindly. Never expose session identifiers in
    aggregate UI labels.
  - **Route:** feature `expose-agent-state-source` after `improve-integration-settings-ux`.
- **STOP:** Stop if the proposed field is named after Settings/sidebar UI, if report age becomes an
  undocumented heartbeat, or if PaneInfo and AgentInfo would disagree for the same terminal.
- **Maintenance:** New lifecycle integrations must declare whether their reports are exclusive,
  session-only, or mixed and add source-projection tests.

### R-02: Full in-app configuration editor

#### What exists

- The generated reference currently describes 153 keys across 13 sections
  (`docs/next/website/src/data/config-reference.json`). The schema includes nested command arrays,
  multiple keybinding syntaxes, colors, platform behavior, advanced/experimental settings, and
  structured sidebar rows.
- Shepherd has no in-app or CLI config editor. It can print the default, print the path, reset
  keybindings, check/reload configuration, and open Settings (`src/main.rs:499-712`,
  `src/cli/spec.rs:157-178`).
- Existing Settings writers intentionally update one scalar key while preserving surrounding text
  (`src/config/io.rs:533-597`). They are not a general TOML document editor.
- There is a cross-platform precedent for opening a file in `$VISUAL`/`$EDITOR` inside an overlay
  pane: scrollback editing at `src/app/input/navigate.rs:883-952` and platform-specific argv
  construction in `src/platform/{linux,macos,windows}.rs`.

#### Options considered

1. **Model all configuration fields as TUI controls.** Rejected: it duplicates a large, evolving
   schema and creates poor controls for commands, arbitrary arrays, custom tokens, and advanced
   platform options.
2. **Embed a raw TOML text editor widget.** Rejected: Shepherd would be rebuilding terminal editor
   behavior while providing less capability than the user's editor.
3. **Keep curated Settings and add an advanced editor bridge.** Selected direction.

#### Decision

Do not build a literal full config editor. Surface stable everyday options as native controls, and
provide an `edit config.toml` advanced action that opens the actual server-side file in the user's
editor, preserves it on syntax errors, reloads after editor exit, and returns actionable diagnostics.
This also works conceptually for remote clients because the editor pane runs where the Shepherd
server and config live.

- **Suggestion:** Add a server-owned advanced config editor bridge rather than an in-TUI TOML
  editor.
  - **Reasoning:** It makes every configuration key reachable without duplicating 153 field
    editors and builds on the existing editor-overlay pattern.
  - **Definition of Done:** Mechanical: neutral platform editor argv helper, Settings/global-menu
    action, process-exit reload, and error diagnostics; behavior: edits the resolved server config,
    keeps current runtime state on invalid TOML, and allows immediate correction; done-when:
    proposal archive, Linux/macOS tests, Windows-target tests, docs, and `just check` pass.
  - **Watch out:** Do not delete the real config as scrollback cleanup deletes its temporary file.
    Preserve quoting and argv boundaries. Decide the neutral server/client command before wiring a
    TUI-only action.
  - **Route:** feature `open-advanced-config-editor`, after responsive Settings and curated UI
    preferences.
- **STOP:** Stop if the design requires sending a client-local path to a remote server, shells an
  unquoted path, rewrites the entire TOML file, or reloads invalid content over the current state.
- **Maintenance:** New config sections remain documented in the generated reference; they do not
  automatically require a new native Settings control.

### R-03: Topbar and sidebar row composer

#### What exists

- Agent rows support eight built-ins, arbitrary `$custom` tokens, and per-occurrence foreground,
  bold, and dim styles (`src/config/sidebar.rs:96-118,152-230,280-304`).
- Space rows have a different five-token vocabulary (`src/config/sidebar.rs:120-132,307-348`).
- Agent sidebar layouts also support per-agent overrides and row gaps
  (`src/config/sidebar.rs:350-432`).
- Topbar rows reuse the Agent token vocabulary (`src/config/topbar.rs:1-28`).
- Layouts may contain up to 16 rows and 16 tokens per row (`src/config/sidebar.rs:7-35`).
- The archived configurable-chrome design rejected a full token editor because it was a much
  larger input/config task (`openspec/changes/archive/2026-08-04-surface-configurable-chrome/design.md:113-117`).

#### Options considered

1. **Full arbitrary visual composer immediately.** Rejected: it needs row/token reordering,
   per-agent inheritance, custom token entry, style editing, validation, preview, and safe
   round-tripping of user-authored arrays.
2. **Built-in token toggles that silently flatten custom layouts.** Rejected: it would destroy
   styles, custom tokens, or per-agent overrides.
3. **Preset-first layouts with custom-safe detection and live preview.** Promising, but preset
   content is a product choice that still needs a small decision map.

#### Decision map

- Offer separate presets for topbar, Agent rows, and Space rows; do not assume one vocabulary fits
  every surface.
- Detect any styled token, custom token, nonzero gap, or per-agent override as `custom`.
- Never overwrite `custom` implicitly. Choosing a preset must preview the exact resulting rows and
  explicitly confirm replacement. Agent presets must leave `rows_by_agent` untouched unless the
  confirmation names and opts into clearing it.
- Keep arbitrary styles, `$custom` names, and per-agent override authoring in TOML for the first
  version.
- Validate candidate presets with actual sidebar widths and a representative topbar rather than
  inventing names from code alone.

- **Suggestion:** Research and validate a preset-first presentation-layout picker before proposing
  implementation.
  - **Reasoning:** The rendering/config substrate is ready, but the useful presets and overwrite
    semantics are not evidenced by current code.
  - **Definition of Done:** Mechanical: record 3-5 candidate layouts with rendered 18/24/36-column
    sidebar and 40/80-column topbar examples; behavior: maintainers can select presets and exact
    custom-preservation rules; done-when: a decision map either routes a bounded feature or records
    why TOML remains sufficient.
  - **Watch out:** Avoid a fake composer that only handles the default shape but destroys advanced
    configurations.
  - **Route:** research/decision map `presentation-layout-presets`, after everyday UI preferences.
- **STOP:** Do not author an OpenSpec implementation proposal until preset contents, preview
  behavior, and custom-layout replacement semantics are approved.
- **Maintenance:** New tokens or style fields must keep custom detection conservative.

### R-04: Agent panel sort placement

#### What exists

- The expanded Agent panel already renders `grouped` or `priority` in its header
  (`src/ui/sidebar.rs:84-105,1522-1545`).
- Clicking the label toggles the ordering and resets scroll
  (`src/app/input/mouse.rs:606-613`); the outer input path persists the change immediately
  (`src/app/input/mod.rs:392-395`, `src/app/config_io.rs:156-174`).
- The control is mouse-only, is labeled with only its current value, and is not explained in the
  user documentation.

#### Decision

Retain the contextual header toggle, but do not treat that as sufficient discoverability or
keyboard access. Include `agent_panel_sort` in `surface-everyday-ui-preferences` and clarify the
header label to `sort: grouped` / `sort: priority`. Both surfaces must write the same config key.
This is intentional contextual duplication, not a second state source.

- **Route:** attach to feature `surface-everyday-ui-preferences`; no standalone sort feature.
- **STOP:** Do not introduce separate persisted sort state for Settings and the sidebar.

### R-05: Settings navigation capacity

#### What exists

- Workspace tabs already solve the same geometry class with a pure calculated viewport, active-tab
  following, clipped hit areas, and optional mouse chevrons (`src/ui/tabs.rs:42-255`).
- Reusable vertical scrollbar calculations exist in `src/ui/scrollbar.rs:14-153` and are already
  used by navigator, release notes, keybind help, and both sidebar lists.
- Settings alone relies on Ratatui `Tabs` with no viewport model and has no narrow render tests.

#### Decision

Responsive Settings is not optional polish and should not remain deferred. It is the prerequisite
for integration management and any new Behavior section. Use one pure section-navigation view
model shared by rendering and hit-testing. Keep the selected section visible; use chevrons or a
compact single-section label on narrow widths. Use the shared scrollbar geometry for long content.

- **Route:** first task of feature `improve-integration-settings-ux`.
- **STOP:** Do not add a seventh fixed-width tab or calculate render and mouse geometry separately.

## Selected suggestion contracts

### Feature: `improve-integration-settings-ux`

- **Suggestion:** Make Settings responsive and turn Integrations into a selectable, scrollable,
  per-target manager.
- **Reasoning:** UI-01 through UI-03 are directly reproducible from current geometry and input;
  neutral per-target operations already exist.
- **Definition of Done:**
  - Mechanical: responsive section viewport; integration row identity/selection/scroll; bounded
    result summary with reachable detail; per-target install/update/uninstall; confirmation for
    uninstall; target version/path detail; bulk install retained as secondary action.
  - Behavior: every target and every failure is reachable at 40x20, 64x20, and 80x24; selected
    rows remain visible; keyboard and mouse act on the same target; results refresh status.
  - Done-when: focused Settings/integration tests and `just check` pass, `docs/next` is updated, and
    the OpenSpec change is archived through the canonical workflow.
- **Watch out:** Reuse neutral `IntegrationTarget` operations; do not introduce Settings-specific
  socket messages or label installation as runtime health.
- **Exemplar:** Display row identity and scroll at `src/app/state.rs:1837-1990`; workspace tab
  overflow at `src/ui/tabs.rs:42-255`.
- **Ordered verification:** add failing 40/64/80 render and input tests; implement geometry; add
  per-target actions; add partial-failure tests; run `just test-one settings`, relevant integration
  tests, then `just check`.
- **Maintenance:** Every new integration target must prove reachability and visible failure output
  at the smallest supported Settings size.

### Feature: `surface-everyday-ui-preferences`

- **Suggestion:** Surface stable visual and interaction preferences after responsive Settings
  lands.
- **In scope:** pane borders, pane gaps, hide-single-tab-bar, close confirmation, tab/workspace name
  prompts, copy-on-select, scroll speed, and Agent panel sort. Visual controls remain under Display;
  interaction controls may use one Behavior section.
- **Out of scope:** full keybinding editor, mouse-capture disablement, host cursor, right-click
  passthrough, startup-only sidebar behavior, token composer, custom theme colors, and arbitrary
  advanced/experimental keys.
- **Definition of Done:** Keyboard/mouse parity; one-key comment-preserving persistence; live reload;
  malformed-config fallback; dynamic row scrolling; 40/64/80-column tests; docs and `just check`.
- **Watch out:** If a field cannot honestly apply live, leave it in TOML or label next-launch
  behavior explicitly.
- **Route:** proposal lane after `improve-integration-settings-ux`.

### Ad-hoc: `render-effective-onboarding-shortcuts`

- **Suggestion:** Render configured prefix/help/settings bindings through shared formatters.
- **Definition of Done:** Default and custom-binding render tests, including a 40-column case, and
  `just check` passing; no keybinding semantics or onboarding flow changes.
- **STOP:** Wrap into a multi-line shortcut block rather than truncate a long binding.

### Ad-hoc: `correct-hermes-next-docs`

- **Suggestion:** Correct unreleased Hermes authority, behavior, and version documentation.
- **Definition of Done:** English, Japanese, and Chinese next docs consistently describe session
  identity plus screen-manifest state; version matches `HERMES_INTEGRATION_VERSION`; translation
  parity and docs checks pass.
- **STOP:** Do not change stable docs without confirming the session-only runtime is in the stable
  release those pages describe.

## Dependency-safe direction

1. Correct Hermes next documentation and onboarding shortcut copy; both are independent ad-hoc
   corrections.
2. Author and implement `improve-integration-settings-ux`.
3. Author and implement `surface-everyday-ui-preferences`, including Agent panel sort parity.
4. Author `expose-agent-state-source`, then add descriptive runtime observations to integration
   management.
5. Author `open-advanced-config-editor` as the advanced escape hatch.
6. Complete the `presentation-layout-presets` decision map; author a feature only if its presets
   and custom-preservation contract are approved.

## Decisions that should not be re-proposed without new evidence

- Do not equate installed/current files with a healthy or active integration.
- Do not build a 153-field in-TUI config form or a raw TOML editor widget.
- Do not ship a token composer that silently flattens styles, custom tokens, row gaps, or per-agent
  overrides.
- Do not add more fixed-width Settings tabs before responsive navigation.
- Do not create a standalone Settings copy of Agent sort with independent state; attach it to the
  shared config-backed preference feature.
- Release, security, and CI improvements were intentionally outside this UI/UX/integration pass.
