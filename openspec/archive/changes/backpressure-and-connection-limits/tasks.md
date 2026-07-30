# Tasks — backpressure-and-connection-limits

Base commit: `1de05dc2`. Drift check: confirm `ClientWriterQueueState.control`
(`src/server/client_transport.rs:189-195`) is still an unbounded `VecDeque`,
`send_control` (`:225`) still `push_back`s unconditionally, and the API accept
loop (`src/api/server.rs:89-107`) still spawns one thread per connection with no
cap. If any is already bounded, STOP and report.

Exemplar: the capacity-1 `render` slot in the same file
(`src/server/client_transport.rs:45-52,192`) — the in-repo precedent for
"slow clients cannot build lag"; and the `broken_clients` cleanup at
`src/server/headless.rs:~2375` for the disconnect path.

## Ordered steps

1. **Classify control messages.** Enumerate what flows through `control`
   (shutdown, clipboard, graphics, notify, …) and tag each must-deliver vs
   droppable. Encode the classification explicitly.
   - Gate: `just test-one client_transport` green (behavior unchanged so far).

2. **Bound the control queue.** Add a byte + count high-water mark to
   `ClientWriterQueueState`. In `send_control`, on overflow: drop droppable
   messages (oldest-first) or, if a must-deliver message would be lost, disconnect
   the client via the existing broken-client path with a logged reason.
   - Gate: new test — a client that never drains keeps the queue bounded and gets
     disconnected; shutdown is never dropped. `just test-one client_transport`.

3. **Cap connections (API).** Add an `AtomicUsize` live-connection counter around
   the spawn in `src/api/server.rs:89-107`; above a ceiling, respond
   `too_many_connections` and close.
   - Gate: new test opens > ceiling connections → excess rejected, under-ceiling
     unaffected.

4. **Cap connections (client handshake).** Same guard in
   `src/server/client_accept.rs:19-45`.
   - Gate: `just test-one client_accept` (or the relevant filter) green.

5. **Verify and close.**
   - `just check` passes. done-when: proposal `backpressure-and-connection-limits`
     archived.

## Notes

- Pick a generous ceiling so normal multi-client + agent use is invisible; the
  cap is a safety valve, not a policy limit.
