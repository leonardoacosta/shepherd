## ADDED Requirements

### Requirement: Unreleased design docs are self-contained

An unreleased document under `docs/next/` SHALL carry the substance it describes, and SHALL NOT
defer a stated payload to a path outside `docs/next/` whose lifecycle it does not control.
Referencing a sibling artifact for context is permitted; replacing the document's own content with
a pointer to one is not.

#### Scenario: Deferred payload

- **WHEN** a staged document states that a table, inventory, or evidence set lives in another file
  rather than reproducing it
- **THEN** documentation validation fails until the payload is inlined or the claim is removed

#### Scenario: Pointer to a deleted change directory

- **WHEN** a staged document cites a path under `openspec/changes/` that is absent from both the
  live change set and the archive
- **THEN** the citation is treated as dangling regardless of whether the target was ever committed

### Requirement: Countable documentation claims are machine-checked

Where an unreleased document states an exact count derived from source, a maintenance script SHALL
assert that count against the source of truth, and SHALL run as part of release documentation
validation.

#### Scenario: Method inventory drifts

- **WHEN** a client-facing `Method` variant is added or removed after the document was written
- **THEN** release documentation validation fails and names both the documented count and the live
  count

#### Scenario: Inventory matches

- **WHEN** the documented tiering table covers every client-facing method exactly once
- **THEN** validation passes without requiring the table to be regenerated

### Requirement: Every client-facing method carries a capability tier

The socket capability tiering inventory SHALL assign a tier to every client-facing `Method`
variant, and SHALL record an explicit rationale for each method tiered exec.

#### Scenario: Process-spawning method

- **WHEN** a method spawns a process or influences the argv of one
- **THEN** it is tiered exec with a rationale naming the spawn, by the same rule applied to
  `agent.start` and `layout.apply`

#### Scenario: Newly added method

- **WHEN** a method is added to the wire surface
- **THEN** it appears in the tiering inventory with a tier before the next release
