## ADDED Requirements

### Requirement: Windows CI executes a measured broad unit suite
Shepherd SHALL execute the materially broad, measured passing set of tests from the `shepherd` binary on Windows instead of selecting only tests by the `windows_` name and one module path.

#### Scenario: Portable unit behavior is eligible on Windows
- **WHEN** a unit test in the `shepherd` binary has no unsupported Windows dependency and passes in the maintained Windows environment
- **THEN** the Windows CI gate includes that test regardless of its name or source module

#### Scenario: The broad suite exceeds one job's time budget
- **WHEN** the measured passing suite cannot complete within the practical Windows CI timeout
- **THEN** the gate divides the same aggregate suite into deterministic shards
- **AND** it does not restore the narrow name-based selection

### Requirement: Windows exclusions are explicit and reviewable
Shepherd SHALL exclude a failing Windows unit test only through an exact quarantine entry with a concise classification and reason.

#### Scenario: A test cannot run on Windows
- **WHEN** measurement proves that a test depends on an unavailable Windows facility
- **THEN** its quarantine identifies the exact test and records why the behavior is Windows-inapplicable

#### Scenario: Measurement exposes a product defect or flake
- **WHEN** a test fails because of a Shepherd defect or demonstrated instability
- **THEN** the quarantine identifies the exact test, classifies the failure, and records the follow-up owner
- **AND** one failure does not silently remove unrelated passing tests from the gate

### Requirement: Portable tests compile and run across platform-gated modules
Shepherd SHALL gate test code at the narrowest practical platform-specific dependency so portable assertions are compiled and executed on Windows.

#### Scenario: A Unix-gated module contains portable tests
- **WHEN** a test's logic and dependencies are supported on Windows even though adjacent tests use Unix-only APIs
- **THEN** the portable test is no longer hidden by a module-wide Unix test gate

#### Scenario: A test requires a Unix-only API
- **WHEN** a test directly depends on Unix sockets, signals, permissions, or another unavailable API
- **THEN** that test or its smallest supporting helper remains compile-gated from Windows
