## 1. Re-anchor the root layout

- [x] 1.1 In `src/ui.rs`, split the frame horizontally before vertically: take `sidebar_area` from
      the full `area`, then split the remainder into `topbar_area` and `main_area`. Keep the
      existing `topbar_height` clamp so at least one main-content row survives.
- [x] 1.2 Update `AppState.view` rectangles so `topbar_rect` starts at the sidebar's right edge and
      `sidebar_rect` spans the full terminal height.
- [x] 1.3 Add a geometry test asserting the sidebar rectangle spans row 0 to the last row while a
      topbar row is resolved, and that `topbar_rect.x == sidebar_w`.
- [x] 1.4 Add a geometry test asserting an unresolved topbar leaves `topbar_rect` empty and the
      sidebar rectangle still full height.
- [x] 1.5 Run `cargo test` for the `ui` module and paste the passing output.

## 2. Add the right chrome panel configuration

- [x] 2.1 In `src/config/sidebar.rs`, add a right-panel config carrying `enabled`, `width`, and
      `rows`, with `rows` typed as the existing `AgentSidebarToken` row vector. Do not introduce a
      second token vocabulary.
- [x] 2.2 In `src/app/state.rs`, add the resolved right-panel rows field typed
      `Vec<Vec<AgentSidebarToken>>`, matching `topbar_rows`.
- [x] 2.3 Add a config test proving an unset right panel defaults to disabled and reserves no width.
- [x] 2.4 Run `cargo test` for the `config` module and paste the passing output.

## 3. Render the right chrome panel

- [x] 3.1 Reuse the Agent-sidebar renderer for the right panel. Call the same
      `sidebar::resolved_token_spans` path the topbar uses. Do not copy the resolver.
- [x] 3.2 Allocate the right-panel rectangle in `src/ui.rs` from the same horizontal split that
      produces `sidebar_area`, so both side panels span full height.
- [x] 3.3 Clamp both side widths so at least one main-content column survives, mirroring the
      existing dock clamp.
- [x] 3.4 Add render tests for token resolution, elision of an absent token and its separator, and
      omission of a fully unresolved row.
- [x] 3.5 Add a clamp test with both side panels enabled on a narrow terminal, asserting one
      main-content column remains.
- [x] 3.6 Run `cargo test` and paste the passing output.

## 4. Correct the spec record

- [x] 4.1 Apply the modified topbar requirement so it states the sidebar-anchored render
      relationship.
- [x] 4.2 Apply the added right-panel requirement.
- [x] 4.3 Apply the added requirement separating the dock from chrome panels. Leave dock behaviour
      unchanged; only the spec's description of it changes.
- [x] 4.4 Run `openspec validate anchor-chrome-to-sidebar --strict --no-interactive` and paste the
      clean output.

## 5. Document the change on the unreleased path

- [x] 5.1 Record the topbar's new position and the right-panel configuration in the unreleased
      documentation the `configurable-chrome` capability already requires.
- [x] 5.2 State that existing `[ui.topbar] rows` values stay valid and that only the render region
      moves.

## 6. Verify the complete feature

- [x] 6.1 Run `just check` and paste the passing output.
- [x] 6.2 Start the binary against a workspace with the left sidebar, the topbar, and the right
      panel enabled. Capture rendered evidence showing the sidebar spanning full height and the
      topbar starting beside it.
- [x] 6.3 Capture rendered evidence with a narrow terminal proving one main-content column survives
      with both side panels enabled.
