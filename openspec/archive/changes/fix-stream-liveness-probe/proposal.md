# Make the stream-liveness probe non-destructive

Base commit: `1de05dc2` · Route: proposal · Effort: M · Confidence: HIGH · Category: correctness

## Why

Resolves advisory finding CORRECTNESS-01 (audit against `1de05dc2`).

The Unix disconnect-detection probe for long-lived API streams performs a
**destructive** read and treats any byte present as a closed peer:

- `src/ipc.rs:157-177` (`probe_stream_closed`, `#[cfg(unix)]`):
  ```rust
  stream.set_nonblocking(true)?;
  let mut probe = [0u8; 1];
  let status = match stream.read(&mut probe) {
      Ok(0) => Ok(true),   // EOF — correct
      Ok(_) => Ok(true),   // BUG: a real byte was read, discarded, and reported as "closed"
      Err(err) if matches!(err.kind(), WouldBlock | Interrupted) => Ok(false),
      Err(err) if is_connection_closed_error(&err) => Ok(true),
      Err(err) => Err(err),
  };
  stream.set_nonblocking(false)?;   // clobbers the caller's polling mode unconditionally
  status
  ```
- The Windows sibling (`src/ipc.rs:180-186`) is already correct: it peeks via
  `PeekNamedPipe` (`windows_named_pipe_available`) and only reads when data is
  actually available — a non-destructive model.

This probe is the sole disconnect detector polled on every long-lived stream
(`events.subscribe`, `events.wait`, `agent.prompt`, `pane.graphicsStream` — via
`should_stop_connection` in `src/api/server.rs`). A client that writes on an
open subscription — including one that pipelines two newline-terminated requests
in a single `write` — has its byte silently swallowed and its subscription torn
down as a "disconnect", with no error surfaced on either side. For a documented
public JSON API (`docs/next/api/herdr-api.schema.json`) this presents to
integrators as random stream drops.

## What changes

Replace the destructive Unix probe with a non-destructive peek —
`recv(fd, buf, MSG_PEEK | MSG_DONTWAIT)` on the underlying fd — so `0` means EOF
(closed) and `>0` means "alive, data pending". Stop toggling the blocking flag
as a side effect (restore the caller's original mode, or peek without changing
it). Leave the Windows branch unchanged; it is already the correct model.

## Acceptance

- `just check` passes.
- A regression test writes bytes on a live subscription stream, calls the probe,
  and asserts (a) the probe reports "not closed" and (b) the written byte is
  still readable by the server afterward.
- A test asserts an actually-closed stream still reports closed.

## Out of scope

- The Windows probe (`src/ipc.rs:180+`) — do not touch it.
- The polling cadence / `CONNECTION_POLL_INTERVAL` in `src/api/server.rs` — this
  change is only about the probe's correctness, not how often it runs.
