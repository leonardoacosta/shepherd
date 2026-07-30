# Bound per-client write queues and cap concurrent connections

Base commit: `1de05dc2` · Route: proposal · Effort: M · Confidence: HIGH · Category: correctness + security

## Why

Adjacent follow-on that resolves advisory findings CORRECTNESS-08 and
SECURITY-11 (≡ CORRECTNESS-11), and is a **prerequisite for shipping
`spike-api-pane-output-stream` safely** — a live output stream over a unix
socket is the exact slow-consumer hazard this change contains.

The server already knows slow clients are dangerous for renders, but only
guards the render slot:

- `src/server/client_transport.rs:45-52` — `ClientWriter.render` is documented
  "Capacity is one so slow clients cannot build lag", but `control` is a
  reliable channel with no bound.
- `src/server/client_transport.rs:189-195` — `ClientWriterQueueState.control:
  VecDeque<Vec<u8>>` is an **unbounded** deque; `render` is a capacity-1
  `Option`.
- `src/server/client_transport.rs:225-232` — `send_control` unconditionally
  `push_back`s with no cap and no backpressure signal.
- `src/server/headless.rs:~2368,~2402` — `send_to_all_clients` / `send_to_client`
  push onto `control` for every broadcast/targeted message, and
  `src/server/headless.rs:~1456` routes potentially large `ServerMessage::Graphics
  { bytes }` (Kitty graphics) through `control` too.

Separately, connections are unbounded:

- `src/api/server.rs:89-107` — the accept loop spawns one `std::thread` per
  accepted connection with no ceiling; each then blocks in
  `read_initial_request_line` up to the initial-request timeout.
- `src/server/client_accept.rs:19-45` — same per client handshake.
- `MAX_INITIAL_REQUEST_BYTES` (`src/api/server.rs:34`) and the 5s timeout bound
  each connection's memory but not the number of connections.

A client that stops reading (suspended terminal, SIGSTOP'd, paused namespace)
fills its socket buffer; the writer blocks in `write_all` while the server keeps
appending clipboard forwards, notifications, and graphics payloads to an
unbounded deque — server RSS grows until OOM, killing every session. And a local
process opening connections in a loop exhausts threads/fds and wedges the daemon.
Both are same-uid, which is precisely herdr's threat surface (it hosts agents).

## What changes

1. **Bound the control queue** with a byte/count high-water mark. Distinguish
   must-deliver control messages (`ServerShutdown`) from droppable ones
   (`Clipboard`, `Graphics`, `Notify`). On overflow, disconnect the client with a
   logged reason (reusing the existing `broken_clients` cleanup at
   `src/server/headless.rs:~2375`) rather than growing unboundedly.
2. **Cap concurrent connections** with a counting semaphore / `AtomicUsize`;
   above the ceiling, write an `ErrorResponse { code: "too_many_connections" }`
   and close instead of spawning. Apply to both the API accept loop and the
   client handshake accept loop.

## Acceptance

- `just check` passes.
- A test drives a stalled client (never drains), asserts the server's queue stays
  bounded and the client is disconnected with the logged reason — not that server
  memory grows.
- A test asserts must-deliver control messages (shutdown) are never dropped by the
  overflow policy.
- A test opens connections past the ceiling and asserts the excess get
  `too_many_connections` and are closed, and that legitimate use under the ceiling
  is unaffected.

## Out of scope

- The render slot (already capacity-1) — leave it.
- Backpressure *semantics of the output stream itself* — that is designed in
  `spike-api-pane-output-stream`; this change provides the shared per-client
  bound the stream will build on.

## STOP conditions

- If dropping any `control` message class turns out to break clipboard or
  shutdown correctness under existing tests, STOP and report — the drop/keep
  classification is the crux and must be gotten right, not guessed.
