# 02: Earning chores, once a day

**What to build:** Tapping a family member's face on a Today's jobs or Other jobs row earns that chore's pebbles straight away, and their pot updates. A short toast at the top confirms it (for example "🐻 🛏️ +2"). That person's face on that row turns into a ✓, and the same chore can't be earned again by them that day. At local midnight in the household time zone the ✓s reset. Other jobs lists every occasional chore by default, and a search box filters the list as you type.

**Blocked by:** 01

**Status:** ready-for-agent

Spec: [Pebblepot v1](../../pebblepot-v1/spec.md)

- [ ] Tapping a face on a daily or occasional chore writes an earn entry to the log, with the item's name, picture and value copied, and the pot shows the new sum.
- [ ] A second earn of the same chore by the same person on the same local day is refused with an "Already done today" message, and the pot is unchanged.
- [ ] After moving the test clock past local midnight, the chore can be earned again and the ✓ is gone.
- [ ] Other jobs shows every occasional chore. Typing in the search box filters by name.
- [ ] A cross-site POST to an earn command is rejected.
- [ ] Covered by HTTP-seam tests.
