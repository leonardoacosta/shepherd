# Tasks — appstate-view-runtime-split

Base commit: `1de05dc2`. Drift check: confirm `AppState` (`src/app/state.rs:~1417`)
still carries the TUI fields listed in the proposal (`sidebar_*`, `*_scroll`,
`toast`, `Mode`, `view: ViewState`) and that `ViewState` (`:~775`) still holds
pixel hit-test geometry. If a split has begun, re-scope to the remaining surface.

Exemplar: the existing `ViewState`/`compute_view_internal` seam (`src/ui.rs`) and
the AGENTS.md refactor-risk process — `AppState::assert_invariants_for_test()`,
`AppState::test_with_adversarial_identity_state()`. This is a roundtable-level
change per AGENTS.md.

## Stage 0 — protect (do before any motion)
1. Name every observable contract that touches these fields: restore/handoff
   round-trip, foreground-client arbitration, TUI rendering. Add/point to
   characterization tests for each.
   - Gate: those tests green on the unchanged code.

## Stage 1 — freeze ViewState
2. Stop new fields landing on `ViewState`; add a `just check` lint or a test that
   fails if the struct grows (pin its field set).
   - Gate: `just check` green; the pin test fails if a field is added.

## Stage 2 — extract ClientViewState
3. Introduce `ClientViewState` owning `ViewState` + `sidebar_*` + `*_scroll` +
   `mouse_*` + `Mode`, held per-client. Move field access off `AppState` behind it,
   compiler-driven, in small commits.
   - Gate after each commit: `just check` + restore/handoff characterization tests
     green.
4. Add a test exercising the server runtime path with **no** foreground client
   view constructed (proves the runtime no longer requires TUI geometry).

## Stage 3 — expose shared facts neutrally
5. For anything the server genuinely must share, expose it via the JSON API/event
   path with neutral names (not `sidebar`/`toast`/`card`). Coordinate with
   `spike-api-pane-output-stream` on shape.
   - Gate: API schema tests (`src/api/schema/tests.rs`) updated + green.

6. **Verify and close.** `just check` passes at each stage. done-when: proposal
   `appstate-view-runtime-split` archived after Stage 2 lands (Stage 3 may be its
   own follow-up).

## STOP conditions
- Any persisted-state or protocol-ID change → STOP, spin a migration proposal with
  a `PROTOCOL_VERSION` decision.
- A field assumed TUI-only that the runtime reads → STOP, capture the coupling; it
  may be a shared fact for the API, not a client field.
