# 05: Live sync across devices

**What to build:** A tap on any device shows up on every other open board within a moment:
- **Server side:** one SSE stream. After every committed write, the server sends a `changed` event whose data is a non-empty version number. A keep-alive goes out every 15 seconds.
- **Client side:** the board refetches when it gets `changed`, when the page becomes visible again (the iPad waking up), and after reconnecting. If the stream errors, for example during a restart, the client re-opens it itself.
- **Shutdown:** a cancellation token closes all streams, so a container stop is quick.

**Blocked by:** 02

**Status:** resolved

Spec: [Pebblepot v1](../../pebblepot-v1/spec.md)

- [x] An open SSE stream receives a `changed` event with non-empty data after an earn (and, later, after any other command).
- [x] Two browser windows stay in sync: earning in one updates the other's pots and ✓s without a manual reload.
- [x] After the server restarts, an open board reconnects and refreshes by itself.
- [x] Shutting down the server closes open streams immediately, so the process exits without waiting out a timeout.
- [x] The `changed` event is covered by an HTTP-seam test.

## Comments

**2026-10-15, implemented** (merged to main in PR #1, e88210d):
- **Server.** `AppState` holds a `watch` version counter and the shutdown `CancellationToken`. One middleware bumps the version after any non-GET/HEAD/OPTIONS request that answers 2xx, so later commands (undo, spend, goals…) are covered with no per-handler code. A refused earn (200 with a refusal toast) bumps too, which is harmless. `GET /events` streams `event: changed` with the version as data (bursts coalesce, keep-alive every 15 s) and ends when the token is cancelled. `GET /board?q=` returns pots plus every section's rows, all out of band, with Other jobs filtered by `q`.
- **Shutdown.** `main` creates the token, cancels it on SIGTERM/Ctrl-C and passes it to `with_graceful_shutdown`. `build_app` takes it as a third argument. The binary exited about 30 to 40 ms after SIGTERM with streams open.
- **Client.** htmx-ext-sse 2.2.4 is vendored (the npm tarball's sha1 and sha512 matched the registry metadata). A hidden `#live` element connects with `sse-connect` and refetches `/board` on `sse:changed`, including the search box, so a typed search is kept. `static/live.js` (about 20 lines) also refetches when the stream (re)opens and on `visibilitychange`/`pageshow`. On wake, it re-opens a stream that was closed without an error. The extension already retries failed streams with backoff. Out-of-band swaps that would change nothing are skipped, so a device's own taps don't flicker.
- **Tests:** 7 new HTTP-seam tests in `tests/live_sync.rs`, 31 in total. Mutation checks: with no bump, 2 tests fail. With no `take_until`, the shutdown test fails. Bumping on GETs as well fails 3 tests.
- **Checked in a browser:** an earn from another client updated the pots and ✓ with the search kept. After a server restart, the page reconnected and kept syncing. A stream that was force-closed re-opened on wake.

**2026-10-15, merged with ticket 03** (built in parallel). The overlapping lines in `src/web.rs` and `templates/board.html` (imports, routes, path constants, the board refresh handler, the `#live` element next to 03's `#sheet`) were resolved by hand. Added `an_undo_also_reaches_every_open_board`, which checks that the write middleware covers ticket 03's `/undo` with no extra code. Checked in a browser: an earn from a second client updated the open board live, and an undo tapped on the board followed by a refresh reached it as well.
