## Context

`PluginSourceInfo` already stores a GitHub plugin's owner, repository, subdirectory, requested ref, resolved commit, managed path, and installation time. `InstalledPluginInfo.version` stores the manifest version. That provenance is sufficient to ask whether a tracked upstream ref now resolves to a different commit; no persistence change is needed.

The marketplace is a GitHub topic-search discovery cache with no authoritative plugin version or commit. It cannot compare installed provenance and it is not a reviewed release channel. Automatically pulling later upstream commits would therefore grant new third-party code execution without the install-time human preview.

The accepted increment is the shipped manual `plugin outdated` report. The user approved the conservative policy: retain explicit user control and defer every unattended or mutating lifecycle feature to a separate security-reviewed proposal.

## Goals / Non-Goals

**Goals:**

- Report drift for installed GitHub plugins that follow a branch, tag, or default `HEAD`.
- Give deterministic outcomes for local, pinned, legacy, ambiguous, and failed comparisons.
- Keep failures isolated per plugin and preserve a successful report sweep.
- Prove old source-less registry JSON remains loadable.

**Non-Goals:**

- Fetching, checking out, installing, or executing new plugin code.
- Automatic update, background polling, passive TUI notices, or new server/API state.
- Treating marketplace metadata as an authoritative release source.

## Decisions

### Compare recorded provenance with one live ref query

For each eligible GitHub plugin, run exactly one `git ls-remote` against the recorded branch/tag or `HEAD`, then compare the returned commit with `source.resolved_commit`. Annotated tags prefer the peeled commit. The query transfers ref metadata only; no repository checkout or code fetch occurs.

### Make non-comparable states explicit

Local plugins report `n/a` without network access. Exact commit refs report `pinned` without network access. Missing recorded commits, missing or ambiguous remote refs, and query failures report `unknown` with detail. A single unknown result does not fail the complete sweep.

### Treat drift as information, not command failure

`current` and `outdated` are successful report states, so the default exit code remains zero. Usage errors and an explicitly unknown `--plugin` retain error exits. A future CI gate would need a separately proposed flag instead of changing this user-facing default.

### Accept the manual-only lifecycle policy

There is no reviewed, versioned plugin channel capable of authorizing automatic updates. Shepherd therefore does not poll, download, install, or execute later upstream code automatically. A passive notice or update mechanism requires a separate proposal covering network privacy, trust, permissions, user confirmation, server/API ownership, and TUI presentation.

## Risks / Trade-offs

- **Live refs can fail or be ambiguous** → Report `unknown` per plugin with labeled detail; preserve the rest of the sweep.
- **Users may interpret `outdated` as unsafe or broken** → Keep it informational and avoid a nonzero default exit solely for drift.
- **Manual checks contact third-party remotes** → Run only on explicit invocation and issue at most one query per eligible plugin.
- **No automatic remediation** → Preserve the existing install preview and user decision as the security boundary.

## Migration Plan

The registry schema is already migration-safe through defaults. Verify the source-less JSON regression, CLI comparison tests, and repository-wide checks; then archive this completed change so the manual drift contract becomes canonical.

## Open Questions

None for this accepted scope. TUI notices, background checks, and any update command are future product and security decisions.
