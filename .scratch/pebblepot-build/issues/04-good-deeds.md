# 04: Good deeds

**What to build:** Tapping a face on a 💛 Kind things row earns that good deed. Good deeds can repeat during the day, but a second identical tap (same person, same good deed) within 2 seconds is refused with "Already counted 👍", which protects against double-taps. Each face on a good deed's row shows a badge with that person's count for today. Tapping the badge takes one away, undoing the most recent.

**Blocked by:** 03

**Status:** ready-for-agent

Spec: [Pebblepot v1](../../pebblepot-v1/spec.md)

- [ ] Earning a good deed twice, more than 2 seconds apart, writes two entries, and the badge shows 2.
- [ ] An identical earn within 2 seconds by the clock is refused, and the pot is unchanged.
- [ ] Tapping the badge removes that person's most recent entry for that good deed today, and the badge decreases (it disappears at 0).
- [ ] Good-deed entries appear in the today list and can be undone there too.
- [ ] Covered by HTTP-seam tests.
