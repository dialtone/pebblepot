# 02: Earning chores, once a day

**What to build:** Tapping a family member's face on a Today's jobs or Other jobs row earns that chore's pebbles straight away, and their pot updates. A short toast at the top confirms it (for example "🐻 🛏️ +2"). That person's face on that row turns into a ✓, and the same chore can't be earned again by them that day. At local midnight in the household time zone the ✓s reset. Other jobs lists every occasional chore by default, and a search box filters the list as you type.

**Blocked by:** 01

**Status:** ready-for-agent

Spec: [Pebblepot v1](../../pebblepot-v1/spec.md)

- [x] Tapping a face on a daily or occasional chore writes an earn entry to the log, with the item's name, picture and value copied, and the pot shows the new sum.
- [x] A second earn of the same chore by the same person on the same local day is refused with an "Already done today" message, and the pot is unchanged.
- [x] After moving the test clock past local midnight, the chore can be earned again and the ✓ is gone.
- [x] Other jobs shows every occasional chore. Typing in the search box filters by name.
- [x] A cross-site POST to an earn command is rejected.
- [x] Covered by HTTP-seam tests.

## Comments

**2026-10-15, implemented** (not committed yet):
- **Rules.** The new rules module's `earn_chore` runs in one immediate transaction. It checks that the family member and chore are active and that the chore hasn't been earned today. The log entry keeps its own copy of the name, picture and amount. The local day comes from the clock plus the household time zone.
- **Earning.** `POST /earn/chore` (form: `member`, `item`) replies with the tapped row, plus the pots and a toast swapped in out of band, so a typed search isn't reset. A refusal answers 200 with a `data-refusal` toast. A non-chore item answers 404.
- **Search.** `GET /occasional?q=` filters Other jobs on the server (case-insensitive name match) and keeps today's ✓s.
- **Tests:** 12 new HTTP-seam tests, 24 in total. They include a test around midnight in Rome (23:59 vs 00:01 local, both the 14th in UTC). Mutation checks showed that removing the daily check fails 3 tests and using UTC instead of the household time zone fails 1.
- **Checked in a browser** on an iPad-sized screen: earn, ✓, the refusal toast, and search followed by a tap.
- **Note for ticket 03:** tapping a ✓ currently just shows "Already done today". Ticket 03 turns it into undo.
- **Note for tickets 01 and 02:** the "item's name, picture and value copied" part of the first criterion is checked through the toast and the pot. It becomes fully observable when edits exist (ticket 09).
