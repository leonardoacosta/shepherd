# Harden the live-handoff channel: token and import_exe

Base commit: `1de05dc2` · Route: proposal · Effort: S · Confidence: HIGH · Category: security

## Why

Resolves advisory findings SECURITY-05 and SECURITY-06 (audit against
`1de05dc2`). Live handoff is how `herdr` self-updates without killing running
agents: it spawns an import server and hands it the session snapshot plus live
pane file descriptors.

Two weaknesses in that channel:

1. **Predictable, argv-exposed token.**
   - `src/server/headless.rs:1052` builds the token as
     `format!("{}-{}", std::process::id(), unix_nanos)` — no CSPRNG, guessable
     (pid is observable, nanos is a narrow window).
   - `src/server/handoff.rs:74-80` passes it as a **command-line argument**
     (`.arg(socket_path).arg(token)`), so it is visible in `ps` /
     `/proc/<pid>/cmdline` to other local accounts for the child's lifetime.
   - The token is the only application-level authentication on a channel that
     transfers the whole session and live PTY fds.

2. **Unconstrained exec sink.**
   - `src/api/schema/server.rs` — `ServerLiveHandoffParams.import_exe:
     Option<String>` is reachable through the public `server.live_handoff` API
     method.
   - `src/server/headless.rs:1050` takes it verbatim into a `PathBuf`, and
     `src/server/handoff.rs:63-90` (`spawn_handoff_import`) spawns it as a
     detached daemon and hands it the session. Any local process that can reach
     the socket — including every agent herdr hosts — can invoke "run this
     arbitrary binary and give it my PTYs" with no confirmation.

The socket's `0600` mode is what actually protects this today; the token adds the
appearance of a second layer without the substance, and `import_exe` is a
general-purpose exec sink on a surface otherwise scoped to terminal manipulation.

## What changes

- Generate the token from the OS CSPRNG (≥128 bits). `getrandom` is already in
  the dependency tree — check before adding anything (AGENTS.md: don't add
  dependencies without a reason).
- Pass the token via an inherited pipe/fd or an environment variable rather than
  argv, so it does not appear in the process table. Update the import child's
  read site (`src/server/headless.rs:~4242`, currently `args[4]`).
- Compare tokens with a constant-time equality helper (`src/server/handoff.rs:~150`).
- Validate `import_exe` before spawn: require an absolute path to a regular file
  (reject symlinks), and constrain it to the directory of
  `std::env::current_exe()` or to a path the update flow just checksum-verified.
  The legitimate caller is `src/update.rs:~1475`, which passes the freshly
  verified update binary — validation must admit exactly that.

## Acceptance

- `just check` passes.
- `tests/live_handoff.rs` still passes (the update/handoff round-trip is
  unchanged for the legitimate path).
- A test asserts `import_exe` pointing outside the allowed location is rejected
  before spawn.
- A test asserts the token is not present in the spawned child's argv.

## Out of scope

- The handoff protocol framing / fd-passing mechanism itself.
- Broader API authentication (peer-credential checks) — tracked separately; this
  change only hardens the handoff method.
