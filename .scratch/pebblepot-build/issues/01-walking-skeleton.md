# 01: Walking skeleton: the family board shows the starting content

**What to build:** Running `cargo run` with no setup starts Pebblepot at http://localhost:8070. On first start it creates the database and seeds it with the [starting content](../../pebblepot-v1/starting-content.md): Kid 1, Kid 2, Mom and Dad, plus every chore, good deed and reward. The family board shows the top strip and all three sections, read-only for now:
- **Top strip:** a pot card per family member (face, name underneath, pebble dots and the number, all 0) and a 🎁 Spend button.
- **Sections, in order:** ☀️ Today's jobs, 💛 Kind things, 🧺 Other jobs. Each row has its picture, name, value dots and a face button per family member.

This ticket also builds the foundations every later ticket uses:
- config defaults
- schema and migrations
- the controllable clock and household time zone
- cross-site request protection
- htmx vendored and served
- the in-process HTTP test harness (fresh temporary database, injected clock), with templates exposing stable semantic markers for tests

**Blocked by:** None (can start immediately)

**Status:** ready-for-agent

Spec: [Pebblepot v1](../../pebblepot-v1/spec.md)

- [ ] `cargo run` with no environment variables serves the board on 127.0.0.1:8070 and creates `./data` if it's missing.
- [ ] Listen address, port, data directory and time zone can each be overridden with environment variables. Setting the listen address to `0.0.0.0` makes the board reachable from the iPad on the LAN.
- [ ] On an empty database, the seed loads the four family members and all items from starting-content.md, in order. Restarting doesn't seed again.
- [ ] The board renders the top strip and all three sections, laid out for iPad landscape and portrait and usable on a phone.
- [ ] A test harness builds the app in-process against a temporary database with a controllable clock. The first tests cover the seed loading once and the board listing every seeded family member and item.
- [ ] Cross-site request protection is installed on the app.
