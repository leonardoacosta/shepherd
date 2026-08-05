Read `CONTEXT.md` and `openspec/changes/archive/*-session-provider-status/` first. This change
multiplies a layering that is already proven; it does not redesign it.

## 1. Settle the `shepherd-state` disposition

- [ ] 1.1 Present options A (one aggregating adapter), B (dissolve the plugin into per-source
      adapters), and C (invoke the plugin once per source) from `proposal.md` `## Decisions`, with
      the independence finding that motivates the choice, and record the answer in `## Decisions`
      as `decided-by: leo`. Every task below depends on it.
- [ ] 1.2 If the answer is B, record in this file which credential-reading code moves into
      Shepherd and what its permissions contract becomes. Do not begin that move before 1.1.

## 2. Split the single adapter into per-source adapters

- [ ] 2.1 Replace `session_status_for_checkout`'s single adapter invocation with one invocation
      per source, each gated on its own demand disjunct.
- [ ] 2.2 Add one parser per source in `src/workspace/session_status.rs`, each pure and testable
      against captured output. Reuse `run_provider` for the command sources; do not copy it.
- [ ] 2.3 Add one optional field per source to the snapshot so each elides independently.
- [ ] 2.4 Add the independence test: one failing adapter and one succeeding adapter in the same
      snapshot resolve to an absent token and a present token respectively.
- [ ] 2.5 Add per-failure-class degradation tests matching each adapter's mechanism — the spawn
      classes for command sources, the read classes for file sources, including a
      partially-malformed file source.
- [ ] 2.6 Run `cargo nextest run session` and paste the passing output.

## 3. Add the vocabulary each source backs

- [ ] 3.1 Add the token names from `proposal.md` `## Impact` to `AgentSidebarToken`, following
      `spend`.
- [ ] 3.2 Extend demand resolution so each token enables only its own adapter, and add a demand
      test per token.
- [ ] 3.3 Resolve each token in `src/ui/sidebar/tokens.rs` through the accessor. Render is a pure
      read; no adapter runs on the render path.
- [ ] 3.4 Add config tests: each name parses, round-trips, and an unknown name is a configuration
      error rather than a silent elision.
- [ ] 3.5 Add render tests asserting independent elision and omission of a fully unresolved row.
- [ ] 3.6 Update `docs/next/website/src/data/config-reference.json` with every new token name.
      `scripts/test_config_reference_check` gates this and `session-provider-status` was caught by
      it.
- [ ] 3.7 Run `cargo nextest run` and paste the passing output.

## 4. Land the dependency amendment

- [ ] 4.1 In `shepherd-plugins`, amend `shepherd-chrome`'s requirement "Chrome values reuse
      metadata transport" so its prohibition scopes to the feed's own projection path and does not
      forbid a consumer resolving a value for itself. Cite this proposal.
- [ ] 4.2 Confirm the amended requirement and this change agree: the feed still MUST NOT add a
      second transport of its own, and Shepherd pulling is not the feed adding one.

## 5. Document

- [ ] 5.1 Document each new token in `docs/next/website/src/content/docs/` (EN, JA, ZH together),
      stating that Shepherd resolves it itself and that an unavailable source elides only its own
      tokens.
- [ ] 5.2 Add a `docs/next/CHANGELOG.md` entry naming the per-source independence.
- [ ] 5.3 Run `just check` and paste the passing output.
- [ ] 5.4 Capture rendered evidence with one source available and another unavailable in the same
      row, and paste the observed row.
