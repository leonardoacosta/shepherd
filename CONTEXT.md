# Shared Vocabulary

Terms this project uses precisely, and the loose words that map onto them. When a request uses a
word from the **Trigger words** column, stop and confirm which layer is meant before writing code
or authoring a proposal — every one of these has been the source of a real misunderstanding.

## Layering

Shepherd resolves a displayed fact through seven layers. `space-project-status` implements all
seven and is the reference to copy.

| Layer | What it is | Reference implementation |
| --- | --- | --- |
| **Domain type** | The shape of the cached fact. Pure data. | `ProjectStatusSnapshot`, `ProposalCounts`, `BeadCounts` — `src/workspace/project_status.rs` |
| **Store** | The cached value, held on a pure-data object inside `AppState`. | `Workspace.cached_project_status`, `Workspace.cached_project_status_key` |
| **Scheduling state** | In-flight and last-run bookkeeping, held on the runtime `App`, never on `AppState`. | `App.last_project_status_refresh`, `App.project_status_refresh_in_flight` |
| **Business layer** | Resolves demand, dedupes work, invokes adapters, parses results, and **writes the store**. Runs off the render path. | `src/app/project_status_refresh.rs` |
| **Provider adapter** | Talks to exactly one source and returns `Option<T>`. Every failure — binary absent, spawn error, non-zero exit, timeout, unparseable body — returns `None`. | `run_provider(cwd, "openspec", ["list", "--json"])` |
| **Accessor** | A pure typed read. No computation, no spawning. | `Workspace::project_status()` |
| **Projection** | Render reads the accessor and draws. Never computes, never spawns, never mutates. | The space token resolver |

Two invariants worth stating separately, because both have been violated in discussion:

- The **store lives in `AppState`**, on the domain object that owns the fact — not on disk, not in
  a plugin, not in a sidecar process. That is what makes it testable without PTYs and what lets
  render stay pure.
- **Shepherd pulls.** The business layer decides when work happens, keyed by demand from the
  configured rows. A design where an outside process pushes values in on its own schedule is the
  thing being moved away from — it leaves shepherd unable to distinguish "absent" from "broken".

## Trigger words

| You might say | You probably mean | Confirm before proceeding |
| --- | --- | --- |
| "business logic", "business layer" | The `src/app/*_refresh.rs` module | Which layer — the refresh module, or the adapter it calls? |
| "global state", "global store", "the store", "state store" | The cached field on a domain object in `AppState` | Not a disk store and not a plugin snapshot — confirm it is `AppState` |
| "external provider", "third-party provider" | Usually a **first-party** source we own | `shepherd-state`, `llmtrim`, `openspec`, and `bd` are all ours. Ask whether "external" means "another process" or "not ours" — they are different claims |
| "first-party provider" | A provider adapter | Which sources, specifically? llmtrim / anthropic / claude / codex / pi / openai are separate adapters |
| "native", "compute it in shepherd" | Shepherd owns the store and business layer | Does it also mean the adapter must not spawn a process? Those are independent choices, one per adapter |
| "absorb the responsibilities" | Move store + scheduling + business layer into shepherd | Which of the seven layers move, and which stay where they are |
| "mimic the sidebar" | Adopt the seven-layer pattern above | Note the sidebar still spawns `openspec` and `bd` — it is not provider-free |
| "topbar" | `[ui.topbar]`, rendered beside the sidebar across main content | Distinguish from the **tab bar** (`src/ui/tabs.rs`) and the **status line** — screenshots have conflated all three |
| "statusline" | Usually the thing being replaced, not a shepherd surface | cc-statusline, cc-tmux, and llmtrim statuslines are external to shepherd |
| "the border against the main body" | The edge adjacent to main content | For the sidebar that is its **right** edge; naming it "left" has meant the main body's left edge. Confirm which element owns the border |
| "sidebar state" | Ambiguous | `AppState` fields, `ClientViewState` projection, or persisted snapshot — three different lifetimes |

## Boundary reminder

`CLAUDE.md` § Runtime/client boundary guardrail still governs. Classify before adding anything:
a shared runtime/session fact belongs in server state and the JSON API; presentation belongs in
the TUI client. The seven layers above describe **where a fact is computed and cached**, not
permission to route it through the private TUI socket.
