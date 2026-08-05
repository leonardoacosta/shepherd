Read `CONTEXT.md` first. This change instantiates its seven layers for session and provider
facts; `space-project-status` is the reference implementation to copy layer for layer.

## 1. Record the baseline

- [x] 1.1 Read `src/app/project_status_refresh.rs` end to end and name, in the PR description,
      the exact function in it that each of this change's new pieces mirrors — demand resolution,
      dedupe, adapter invocation, timeout, parse, store write.
- [x] 1.2 Run `cargo nextest run project_status` and paste the passing output as the pre-change
      baseline for the pattern being copied.
- [x] 1.3 Confirm the current token vocabulary and record it: the built-ins in
      `src/config/sidebar.rs`, what `TopbarConfig` accepts, and which `$custom` metadata tokens
      operators may already be consuming. This change must not break the latter.

## 2. Domain type and store

- [x] 2.1 Add the domain type for the session/provider fact next to
      `src/workspace/project_status.rs`, following its shape: a snapshot struct of `Option` fields,
      defaulting to all-absent.
- [x] 2.2 Add the cached field and its key to the object that owns the fact, mirroring
      `cached_project_status` / `cached_project_status_key`. The store lives in `AppState`.
- [x] 2.3 Add the scheduling counterpart on the runtime `App`, mirroring
      `last_project_status_refresh` / `project_status_refresh_in_flight`. Scheduling never lives
      on `AppState`.
- [x] 2.4 Add the pure accessor, mirroring `Workspace::project_status()`.
- [x] 2.5 Assert the invariants with `AppState::assert_invariants_for_test()` and confirm the
      store round-trips through the existing adversarial identity-state helpers.

## 3. Business layer with one adapter

- [x] 3.1 Add `src/app/session_provider_refresh.rs` mirroring `project_status_refresh.rs`:
      demand resolution per token, dedupe, off-render-path execution, own cadence.
- [x] 3.2 Implement exactly one provider adapter returning `Option`. Choose the source with the
      simplest contract; record in the PR why that one was chosen to prove the layering.
- [x] 3.3 Reproduce the degradation contract exactly: source absent, spawn error, non-zero exit,
      timeout, and unparseable body all return `None` and touch nothing else.
- [x] 3.4 Wire the token into the vocabulary and resolve it through the accessor in
      `src/ui/topbar.rs`. Existing `$custom` metadata tokens keep working unchanged.
- [x] 3.5 Add the tests named in `proposal.md` § Testing: demand, per-failure-class degradation,
      render-path non-blocking, independent cadence, accessor-reads-store.
- [x] 3.6 Run `cargo nextest run` and paste the passing output.

## 4. Structure the remaining adapters

- [x] 4.1 With the layering proven, write the follow-on proposal covering the remaining provider
      adapters — llmtrim, anthropic, claude, codex, pi, openai. For each, record the source it
      reads, which token(s) it backs, its failure modes, and whether it spawns a command or reads
      a file directly. That last choice is per adapter, never global.
- [x] 4.2 In that proposal, record the disposition of `shepherd-state`: whether it becomes one
      adapter shepherd invokes, or its logic migrates in adapter by adapter and the plugin
      dissolves. Do not decide this here — the answer depends on what 4.1 finds.
- [x] 4.3 Name the amendment the follow-on needs to `shepherd-chrome`'s requirement "Chrome values
      reuse metadata transport … MUST NOT add a second chrome transport"
      (`shepherd-plugins/openspec/specs/shepherd-chrome/spec.md`), which shepherd owning the store
      contradicts. Declare it as a dependency of that proposal, not of this one.

## 5. Document

- [x] 5.1 Document the new tokens in `docs/next/website/src/content/docs/` (EN, JA, ZH together),
      stating that shepherd resolves them itself and that an unavailable source elides its token.
- [x] 5.2 Add a `docs/next/CHANGELOG.md` entry.
- [x] 5.3 Run `just check` and paste the passing output.
- [x] 5.4 Manually verify a rendered capture with the provider available and unavailable, and
      paste both observed rows.
