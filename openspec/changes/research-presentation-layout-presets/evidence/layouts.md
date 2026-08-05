# Rendered evidence: presentation layout presets

Source revision: `shepherd@6aad4885c22a07ccaf28df634c407064c8b8c65c` (dev, post
`surface-settings-and-integration-controls`, `expose-agent-state-source`, and
`open-advanced-config-editor` waves).

Generation command (production token resolution + production cell rendering,
not hand-drawn text):

```
cargo nextest run --locked "presentation_layout_candidate" --status-level fail \
  --final-status-level fail --failure-output final --success-output never --no-capture
```

Result: `7 tests run: 7 passed, 3218 skipped` (pasted in full under §5).

Test location: `src/ui/sidebar/tokens.rs::presentation_layout_candidate` (Agent +
Space) and `src/ui/topbar.rs::presentation_layout_candidate` (topbar). Every
row is produced by calling the real `agent_rows_from` / `space_rows` token
resolvers and the real `sidebar::resolved_token_spans` cell renderer — the same
functions `render_agent_detail`, `render_topbar`, and `render_sidebar` call in
the shipped UI — then flattening the returned spans to plain text.

## 1. Inventory confirmation (task 1.1)

`surface-settings-and-integration-controls` is applied and on `dev` (not yet
archived; archival happens at the end of this run per the dispatch). Settings
already declares `SettingsSection::Display` and `SettingsSection::Behavior`
(`src/ui/settings.rs`), but neither currently renders anything sidebar- or
topbar-layout-related (`rg -n "sidebar|topbar" src/ui/settings.rs` returns no
matches) — a future picker has an obvious landing section but nothing to
migrate away from. `expose-agent-state-source` (typed `state_source` on
pane/agent APIs) and `open-advanced-config-editor` (`server.config.edit`
editor pane, `AdvancedConfigEditor` mode) do not touch
`src/config/sidebar.rs`, `src/config/topbar.rs`, or the token resolvers —
zero drift from those two waves. The advanced config editor does, however,
strengthen the "TOML remains sufficient" fallback: operators already have an
in-app editor pane for hand-authoring these exact TOML row arrays without a
picker.

Current schema, reconfirmed against source (no drift from `design.md`'s
Context section):

- **Agent tokens** (`AgentSidebarToken`, `src/config/sidebar.rs`): 8 built-ins
  (`state_icon`, `state_text`, `workspace`, `tab`, `pane`, `agent`,
  `terminal_title`, `terminal_title_stripped`) + `Custom(String)` (`$name`) +
  `Styled { token, style }`.
- **Space tokens** (`SpaceSidebarToken`): 5 built-ins (`state_icon`,
  `state_text`, `workspace`, `branch`, `git_status`) + `Custom` + `Styled`.
- **Topbar** (`TopbarConfig`, `src/config/topbar.rs`): reuses
  `AgentSidebarToken` at a different render width; `TopbarConfig` has only
  `enabled: bool` and `rows`, **no `row_gap` field** — nonzero-gap custom
  classification does not apply to the topbar surface, only to Agent/Space
  sidebars.
- **Style fields** (`SidebarTokenStyle`): `fg: Option<SidebarTokenColor>`,
  `bold: Option<bool>`, `dim: Option<bool>`.
- **Limits** (`MAX_SIDEBAR_ROWS = 16`, `MAX_SIDEBAR_TOKENS_PER_ROW = 16`),
  shared by Agent rows, Space rows, and topbar rows via
  `validate_sidebar_rows`.
- **Row gap**: `AgentsSidebarConfig.row_gap: u16` and
  `SpacesSidebarConfig.row_gap: u16`, default `0`.
- **Per-agent overrides**: `AgentsSidebarConfig.rows_by_agent: BTreeMap<String,
  AgentSidebarRows>`, keyed by canonical agent id, resolved by
  `AgentsSidebarConfig::rows_for_agent`. Space rows have no per-agent override
  path.
- **Elision/truncation rules** (`src/ui/sidebar.rs::resolved_token_spans`,
  `src/ui/text.rs`): a row with zero resolved tokens is dropped entirely
  (`agent_rows_from`/`space_rows` filter it via `filter_map`); when a row's
  minimum width exceeds the budget, flexible (text) tokens are dropped
  right-to-left first, then remaining flexible tokens are truncated with
  `truncate_end` (a trailing `…`, `unicode-width`-aware); fixed-width tokens
  (`state_icon`, `git_status`) are never truncated, only present/absent
  (`git_status` elides entirely when both `ahead` and `behind` are `0`).

## 2. Candidates (task 2.1)

Named exact TOML-equivalent row arrays. Agent, Space, and topbar are separate
families — no shared vocabulary assumed. Baseline = current
`Default::default()` for that surface.

### 2.1 Agent sidebar candidates

```toml
# baseline-compact (current AgentsSidebarConfig::default())
rows = [
  ["state_icon", "workspace", "tab"],
  ["agent"],
]

# single-line
rows = [
  ["state_icon", "workspace", "agent"],
]

# status-first
rows = [
  ["state_icon", "state_text"],
  ["workspace", "tab"],
  ["agent"],
]

# terminal-title
rows = [
  ["state_icon", "terminal_title_stripped"],
  ["agent"],
]

# full-context
rows = [
  ["state_icon", "workspace", "tab"],
  ["pane", "agent"],
]
```

Rejected: a "kitchen-sink" candidate combining `terminal_title_stripped` +
`pane` + `tab` + `agent` in one row — at 18 columns this collapses to a single
truncated token once separators are counted, providing strictly less
information than `full-context` or `terminal-title` at the same width; not
carried forward.

### 2.2 Space sidebar candidates

```toml
# baseline (current SpacesSidebarConfig::default())
rows = [
  ["state_icon", "workspace"],
  ["branch", "git_status"],
]

# single-line
rows = [
  ["state_icon", "workspace", "branch"],
]

# status-first
rows = [
  ["state_icon", "state_text"],
  ["workspace"],
  ["branch", "git_status"],
]

# branch-only
rows = [
  ["state_icon", "workspace"],
  ["branch"],
]
```

Rejected: a `git_status`-only second row (dropping `branch`) — `git_status`
elides completely whenever a space has no ahead/behind delta, which is the
common case for a space sitting on its default branch, leaving that row empty
far more often than `branch` does; not carried forward as a named candidate.

### 2.3 Topbar candidates

```toml
# baseline-workspace-only (current TopbarConfig::default())
enabled = true
rows = [["workspace"]]

# workspace-tab
enabled = true
rows = [["workspace", "tab"]]

# workspace-agent-two-row
enabled = true
rows = [
  ["workspace"],
  ["agent"],
]

# full-context
enabled = true
rows = [
  ["workspace", "tab", "pane"],
  ["agent"],
]

# status-first
enabled = true
rows = [["state_icon", "state_text", "workspace"]]
```

Rejected: a `terminal_title` topbar candidate — the topbar already occupies
the same horizontal band as the terminal's own OS-level title bar / tab strip
on most terminals Shepherd runs inside; duplicating that text a second time
at 40 columns leaves almost no budget for `workspace`, which is the one value
the topbar is uniquely positioned to show. Not carried forward.

## 3. Rendered matrices (tasks 2.2, 2.3)

Test cases per surface (constructed by `presentation_layout_candidate`, not
hand-written): **representative** (short, realistic values, all fields
present), **long** (long workspace/tab/pane/agent/branch names and a
non-ASCII terminal title, to exercise elision + `…` truncation), **missing**
(tab/pane/agent/branch/ahead-behind all absent, to exercise row elision).

Separator is `" · "` between text tokens and `" "` after `state_icon` /
before `git_status`, per `tokens::separator` — visible in every row below.
`⏎` marks a row break within one candidate's rendered card (not part of the
rendered text).

### 3.1 Agent sidebar — 18 / 24 / 36 columns

```
baseline-compact
  representative/18: ● shephe… · featu… ⏎ claude
  representative/24: ● shepherd · feature-au… ⏎ claude
  representative/36: ● shepherd · feature-auth ⏎ claude
  long/18: ● backen… · refac… ⏎ claude-opus-5-son…
  long/24: ● backend-s… · refactor… ⏎ claude-opus-5-sonnet-re…
  long/36: ● backend-service… · refactor-billi… ⏎ claude-opus-5-sonnet-reasoning
  missing/18: · solo-workspace   (tab row elides entirely; agent row elides entirely)
  missing/24: · solo-workspace
  missing/36: · solo-workspace

single-line
  representative/18: ● shephe… · claude
  representative/24: ● shepherd · claude
  representative/36: ● shepherd · claude
  long/18: ● backen… · claud…
  long/24: ● backend-s… · claude-o…
  long/36: ● backend-service… · claude-opus-5-…
  missing/18: · solo-workspace   (agent token drops, row keeps workspace)
  missing/24: · solo-workspace
  missing/36: · solo-workspace

status-first
  representative/18: ● working ⏎ shepherd · featur… ⏎ claude
  representative/24: ● working ⏎ shepherd · feature-auth ⏎ claude
  representative/36: ● working ⏎ shepherd · feature-auth ⏎ claude
  long/18: ● blocked ⏎ backend… · refact… ⏎ claude-opus-5-son…
  long/24: ● blocked ⏎ backend-se… · refactor-… ⏎ claude-opus-5-sonnet-re…
  long/36: ● blocked ⏎ backend-services… · refactor-billin… ⏎ claude-opus-5-sonnet-reasoning
  missing/18: · idle ⏎ solo-workspace   (workspace+tab row keeps workspace; agent row elides)
  missing/24: · idle ⏎ solo-workspace
  missing/36: · idle ⏎ solo-workspace

terminal-title
  representative/18: ● building ⏎ claude
  representative/24: ● building ⏎ claude
  representative/36: ● building ⏎ claude
  long/18: ● 修复用户认证模… ⏎ claude-opus-5-son…
  long/24: ● 修复用户认证模块并迁… ⏎ claude-opus-5-sonnet-re…
  long/36: ● 修复用户认证模块并迁移到统一登录… ⏎ claude-opus-5-sonnet-reasoning
  missing/18: ·   (terminal_title_stripped token drops the whole row; agent row also elides)
  missing/24: ·
  missing/36: ·

full-context
  representative/18: ● shephe… · featu… ⏎ review p… · claude
  representative/24: ● shepherd · feature-au… ⏎ review pane · claude
  representative/36: ● shepherd · feature-auth ⏎ review pane · claude
  long/18: ● backen… · refac… ⏎ review … · claude…
  long/24: ● backend-s… · refactor… ⏎ review pan… · claude-op…
  long/36: ● backend-service… · refactor-billi… ⏎ review pane for … · claude-opus-5-s…
  missing/18: · solo-workspace   (pane+agent row elides entirely — see §6)
  missing/24: · solo-workspace
  missing/36: · solo-workspace
```

`terminal-title/missing` is the sharpest finding in this table: when neither
`terminal_title_stripped` nor `agent` resolve, the card renders **only the
state dot** — no workspace fallback exists in that candidate at all, unlike
every other candidate. This is a real usability gap for that candidate, not a
rendering bug; recorded in §7.

### 3.2 Space sidebar — 18 / 24 / 36 columns

```
baseline
  representative/18: ○ shepherd ⏎ main ↑1
  representative/24: ○ shepherd ⏎ main ↑1
  representative/36: ○ shepherd ⏎ main ↑1
  long/18: ○ backend-service… ⏎ refactor/… ↑12 ↓34
  long/24: ○ backend-services-paym… ⏎ refactor/billin… ↑12 ↓34
  long/36: ○ backend-services-payment-orchestr… ⏎ refactor/billing-reconcilia… ↑12 ↓34
  missing/18: ○ solo-workspace   (branch+git_status row elides entirely)
  missing/24: ○ solo-workspace
  missing/36: ○ solo-workspace

single-line
  representative/18: ○ shepherd · main
  representative/24: ○ shepherd · main
  representative/36: ○ shepherd · main
  long/18: ○ backen… · refac…
  long/24: ○ backend-s… · refactor…
  long/36: ○ backend-service… · refactor/billi…
  missing/18: ○ solo-workspace
  missing/24: ○ solo-workspace
  missing/36: ○ solo-workspace

status-first
  representative/18: ○ idle ⏎ shepherd ⏎ main ↑1
  representative/24: ○ idle ⏎ shepherd ⏎ main ↑1
  representative/36: ○ idle ⏎ shepherd ⏎ main ↑1
  long/18: ○ working ⏎ backend-services-… ⏎ refactor/… ↑12 ↓34
  long/24: ○ working ⏎ backend-services-paymen… ⏎ refactor/billin… ↑12 ↓34
  long/36: ○ working ⏎ backend-services-payment-orchestrat… ⏎ refactor/billing-reconcilia… ↑12 ↓34
  missing/18: ○ idle ⏎ solo-workspace
  missing/24: ○ idle ⏎ solo-workspace
  missing/36: ○ idle ⏎ solo-workspace

branch-only
  representative/18: ○ shepherd ⏎ main
  representative/24: ○ shepherd ⏎ main
  representative/36: ○ shepherd ⏎ main
  long/18: ○ backend-service… ⏎ refactor/billing-…
  long/24: ○ backend-services-paym… ⏎ refactor/billing-reconc…
  long/36: ○ backend-services-payment-orchestr… ⏎ refactor/billing-reconciliation-pip…
  missing/18: ○ solo-workspace
  missing/24: ○ solo-workspace
  missing/36: ○ solo-workspace
```

`baseline/representative` shows `git_status` rendering `↑1` alone (no `↓`) —
confirms the `ahead > 0 && behind > 0` separator space only appears when both
are nonzero (`resolved_token_spans` `GitStatus` arm), not a fixed two-slot
field.

### 3.3 Topbar — 40 / 80 columns

```
baseline-workspace-only
  representative/40: shepherd
  representative/80: shepherd
  long/40:            backend-services-payment-orchestration-…
  long/80:            backend-services-payment-orchestration-monorepo
  missing/40:          solo-workspace
  missing/80:          solo-workspace

workspace-tab
  representative/40: shepherd · feature-auth
  representative/80: shepherd · feature-auth
  long/40:            backend-services-p… · refactor-billing-…
  long/80:            backend-services-payment-orchestration… · refactor-billing-reconciliation-pipel…
  missing/40:          solo-workspace   (tab token drops)
  missing/80:          solo-workspace

workspace-agent-two-row
  representative/40: shepherd ⏎ claude
  representative/80: shepherd ⏎ claude
  long/40:            backend-services-payment-orchestration-… ⏎ claude-opus-5-sonnet-reasoning
  long/80:            backend-services-payment-orchestration-monorepo ⏎ claude-opus-5-sonnet-reasoning
  missing/40:          solo-workspace   (agent row elides entirely)
  missing/80:          solo-workspace

full-context
  representative/40: shepherd · feature-auth · review pane ⏎ claude
  representative/80: shepherd · feature-auth · review pane ⏎ claude
  long/40:            backend-ser… · refactor-b… · review pan… ⏎ claude-opus-5-sonnet-reasoning
  long/80:            backend-services-payment… · refactor-billing-reconci… · review pane for the bil… ⏎ claude-opus-5-sonnet-reasoning
  missing/40:          solo-workspace   (tab+pane drop inside row 1; agent row elides — see §6)
  missing/80:          solo-workspace

status-first
  representative/40: ● working · shepherd
  representative/80: ● working · shepherd
  long/40:            ● blocked · backend-services-payment-or…
  long/80:            ● blocked · backend-services-payment-orchestration-monorepo
  missing/40:          · idle · solo-workspace
  missing/80:          · idle · solo-workspace
```

At 40 columns `full-context/long` already truncates every one of its four
tokens (`workspace`, `tab`, `pane`, `agent`) to single-digit remaining
characters before the ellipsis — the candidate is legible but dense; recorded
as a trade-off in §7, not disqualifying.

## 4. Decision table (task 3.1)

Every scenario has one explicit, conservative outcome. "Custom" means: a
future picker must never silently offer this configuration as an exact
preset match, must never overwrite it without the explicit
base-row-replacement confirmation, and must treat it as `$custom` for
recognition purposes even if its rendered output happens to match a preset.

| Scenario | Outcome |
| --- | --- |
| Config equals a named candidate's rows array exactly (post schema-default normalization: `row_gap` present-and-zero treated same as absent) | Exact preset match — a picker may pre-select that candidate as the current selection. Visual similarity of rendered output is **not** sufficient; only structural equality of the normalized rows array qualifies. |
| Any token in any row is `Styled { .. }` (has `fg`/`bold`/`dim`) | `$custom`. Styling is never absorbed into a preset name. |
| Any token is `Custom(name)` (`$name`) | `$custom`. Custom tokens carry user-authored meaning a preset cannot represent. |
| `row_gap` (Agent or Space) is nonzero | `$custom`. Topbar has no `row_gap` field, so this scenario cannot occur for topbar rows. |
| `AgentsSidebarConfig.rows_by_agent` is non-empty (any entry) | `$custom` for the base `rows`, independent of whether `rows` itself matches a preset. Per-agent overrides are surfaced separately (see next row) and are never silently dropped or matched against a preset. |
| An unrecognized/future config field appears (schema evolves; e.g. a new token variant or a new style field this research did not enumerate) | `$custom` by default, per `design.md`'s conservative-classification decision — a new field must be explicitly added to the classifier before it can ever match a preset. Never silently ignored or dropped. |
| User cancels the (future) picker flow mid-selection | No write. Config on disk/in memory is byte-for-byte unchanged. This is a `design.md`-level contract for any future implementation, not a behavior this research change adds. |
| User confirms replacing a `$custom` base `rows` array with a preset | Requires the explicit "replace custom layout" confirmation named in `design.md`; the confirmation must show the exact resulting rows before it is written. Declining leaves `rows` untouched. |
| User confirms replacing a preset-matching (non-custom) base `rows` array with a different preset | Allowed without the custom-specific warning, but the exact resulting rows are still shown before write (preview-precedes-replacement applies to every replacement, custom or not). |
| User wants to clear `rows_by_agent` when applying a base-row preset | Never implicit. Requires a **separately named** opt-in ("also clear per-agent overrides") in the same confirmation dialog as the base-row replacement — confirming the base-row replacement alone MUST leave `rows_by_agent` untouched. |

## 5. Verification output

```
$ cargo nextest run --locked "presentation_layout_candidate" --status-level fail \
    --final-status-level fail --failure-output final --success-output never --no-capture
...
Summary [   0.028s] 7 tests run: 7 passed, 3218 skipped
```

Seven tests: `agent_candidates_render_within_declared_sidebar_widths`,
`agent_long_values_truncate_with_ellipsis_at_narrow_widths`,
`agent_missing_fields_elide_rows_and_reduce_row_count_versus_representative`,
`space_candidates_render_within_declared_sidebar_widths`,
`space_zero_ahead_behind_elides_git_status_but_missing_branch_elides_row`,
`topbar_candidates_render_within_declared_widths`,
`topbar_missing_tab_and_pane_elide_from_full_context_row`.

```
$ rg -n '\$custom|rows_by_agent|gap|fg|bold|dim' src/config/sidebar.rs src/config/topbar.rs
```

Every field this returns (`fg`, `bold`, `dim`, `rows_by_agent`, `row_gap`) is
covered by a row in §4's decision table; `$custom` is covered via the
`Custom(name)` row. Topbar has no matches for `gap` — confirmed above as the
"topbar has no `row_gap`" drift note.

## 6. Safe-replacement analysis

What a future preset-apply write would touch, and what it must never touch:

- **In scope for overwrite**: the `rows` array of the surface being replaced
  (Agent `rows`, Space `rows`, or `TopbarConfig.rows`) and, for the
  base-row-replacement confirmation, the surface's own `row_gap` back to `0`
  (every candidate above assumes `row_gap = 0`; a candidate never specifies a
  nonzero gap since gap is orthogonal to the row/token content this research
  compares).
- **Never in scope, ever, without a separate named opt-in**:
  `AgentsSidebarConfig.rows_by_agent`. Replacing the base `rows` with a
  preset must not touch any per-agent override, including overrides for
  agents that happen to not be running right now.
- **Never in scope, full stop**: the other surface's config (applying an
  Agent preset must not touch `SpacesSidebarConfig` or `TopbarConfig`, and
  vice versa — the three families are independent, per `design.md`'s
  "candidate families remain separate" decision).
- **`terminal-title`'s missing-value gap** (§3.1): if this candidate is
  approved, its no-workspace-fallback behavior when both
  `terminal_title_stripped` and `agent` are absent should be called out to
  the user at selection time, not silently accepted — this is evidence for
  the terminal gate to weigh, not something this research change fixes.

## 7. Notes for the terminal gate

- All three surfaces have a workable single-line variant (`single-line` for
  Agent, `single-line` for Space) that stays under 24 columns without
  truncating short-to-medium real-world values, and a `status-first` variant
  that trades a dedicated row for `state_text` visibility.
- `full-context` (Agent and topbar) is the densest candidate in both
  families; it is legible at 24+ / 80 columns but truncates aggressively at
  18 / 40. If approved, it is the candidate most likely to need a
  wider-than-default minimum width guard — a product decision, not an
  outcome this research change makes.
- `terminal-title` (Agent) has the missing-value gap noted in §3.1 and §6;
  recommend either pairing it with a `workspace` fallback token before
  approval, or rejecting it as-is.
- Topbar's `terminal_title` candidate was rejected pre-render (§2.3) rather
  than measured, because the topbar already competes for the same visual
  role as most terminals' native title bar; if the terminal gate disagrees,
  it should request that one specific candidate be added and re-measured
  before approval, not accept the rejection on rationale alone.

## 8. Terminal gate disposition (task 5.1)

`decided-by: leo (delegated to the applying agent at apply time)` — recorded as delegated,
open to reversal. Rationale in `design.md` § Terminal gate disposition.

| Surface | Candidate | Disposition |
| --- | --- | --- |
| Agent | `baseline-compact` | Approved (current default; retained so a user can return to it) |
| Agent | `single-line` | Approved |
| Agent | `status-first` | Approved |
| Agent | `full-context` | Approved — dense at 18 columns but retains a usable workspace name |
| Agent | `terminal-title` | **Rejected as-is** — §3.1 `missing` renders the state dot alone, no fallback identity |
| Space | `baseline` | Approved (current default) |
| Space | `single-line` | Approved |
| Space | `status-first` | Approved |
| Space | `branch-only` | Approved |
| Topbar | `baseline-workspace-only` | Approved (current default) |
| Topbar | `workspace-tab` | Approved |
| Topbar | `workspace-agent-two-row` | Approved |
| Topbar | `status-first` | Approved |
| Topbar | `full-context` | **Rejected** — §3.3 `long/40` truncates all four tokens simultaneously |
| Topbar | `terminal_title` | Rejection carried forward on rationale (§2.3); never rendered, so unlike the two above it is not evidence-backed |

Eleven approved, three not. Every surface keeps a picker. No implementation is authored by
this change; a future OpenSpec change may author a picker offering only the approved names.
