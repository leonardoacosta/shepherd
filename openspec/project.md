# OpenSpec — herdr change proposals

This directory holds draft change proposals produced by an advisory audit. Each
subdirectory under `changes/` is one proposed change: a self-contained
`proposal.md` (why + what + acceptance) and `tasks.md` (ordered, verifiable
steps). Proposals are drafts for the maintainer to accept, defer, or reject —
nothing here is source code and nothing here has been applied.

## Contract for executors

Every proposal is written for an executor with **zero context** from the audit
session. Before touching code:

1. Check out the base commit stamped in the proposal (`Base commit:` line) or
   confirm the cited `file:line` sites still match the excerpts shown. **If the
   code has drifted from the excerpt, STOP and report the drift** — do not
   improvise a fix around it.
2. Follow the repo's own rules in `AGENTS.md` (no `unwrap()` in production code,
   platform code isolated in `src/platform/`, `just check` before committing,
   propose the commit message first).
3. Each task's mechanical gate is an exact command with an expected result. Do
   not mark a task done on judgment ("looks right") — run the gate.

## Verification commands (from `justfile`)

- `just lint` — `cargo fmt --check` + `cargo clippy --all-targets --locked -- -D warnings`
- `just test` — `cargo nextest run` + python maintenance-script tests + bun asset/worker tests
- `just check` — `just ci` + windows-target clippy + maintenance tests (run before committing)
- `just test-one <filter>` — single nextest filter

Base commit for every proposal in this batch: **`1de05dc2`** (branch `dev`).
