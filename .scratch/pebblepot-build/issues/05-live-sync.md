# 05: Live sync across devices

**What to build:** A tap on any device shows up on every other open board within a moment:
- **Server side:** one SSE stream. After every committed write, the server sends a `changed` event whose data is a non-empty version number. A keep-alive goes out every 15 seconds.
- **Client side:** the board refetches when it gets `changed`, when the page becomes visible again (the iPad waking up), and after reconnecting. If the stream errors, for example during a restart, the client re-opens it itself.
- **Shutdown:** a cancellation token closes all streams, so a container stop is quick.

**Blocked by:** 02

**Status:** ready-for-agent

Spec: [Pebblepot v1](../../pebblepot-v1/spec.md)

- [ ] An open SSE stream receives a `changed` event with non-empty data after an earn (and, later, after any other command).
- [ ] Two browser windows stay in sync: earning in one updates the other's pots and ✓s without a manual reload.
- [ ] After the server restarts, an open board reconnects and refreshes by itself.
- [ ] Shutting down the server closes open streams immediately, so the process exits without waiting out a timeout.
- [ ] The `changed` event is covered by an HTTP-seam test.
