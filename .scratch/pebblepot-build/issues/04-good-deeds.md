# 04: Good deeds

**What to build:** Tapping a face on a 💛 Kind things row earns that good deed. Good deeds can repeat during the day, but a second identical tap (same person, same good deed) within 2 seconds is refused with "Already counted 👍", which protects against double-taps. Each face on a good deed's row shows a badge with that person's count for today. Tapping the badge takes one away, undoing the most recent.

**Blocked by:** 03

**Status:** resolved

Spec: [Pebblepot v1](../../pebblepot-v1/spec.md)

- [x] Earning a good deed twice, more than 2 seconds apart, writes two entries, and the badge shows 2.
- [x] An identical earn within 2 seconds by the clock is refused, and the pot is unchanged.
- [x] Tapping the badge removes that person's most recent entry for that good deed today, and the badge decreases (it disappears at 0).
- [x] Good-deed entries appear in the today list and can be undone there too.
- [x] Covered by HTTP-seam tests.

## Comments

**2026-10-15, implemented** (built together with 06 on build/tickets-04-06):
- **Rules.** New `rules::earn_good_deed(store, now, tz, member, item)` in one immediate transaction: checks the member is active and the item is an active good deed, then refuses with the new `Refusal::RepeatTap` (`repeat_tap`, "Already counted 👍") if that person's latest earn of that good deed has a `created_at` within `REPEAT_TAP_WINDOW` (2 s, inclusive, either direction so a clock stepped back can't double-count). Refused taps write nothing, so they don't restart the window. `earn_chore` is untouched.
- **Read model.** `Board.good_deed_counts` maps (member, good deed) to `GoodDeedCount { count, latest_entry }` for today's local day. The badge undoes `latest_entry`.
- **Web.** `POST /earn/good-deed` (`EARN_GOOD_DEED_PATH`, form `member`, `item`) answers like the chore earn: the row, plus pots and toast (with ↩︎ Undo) out of band. A non-good-deed item answers 404. Faces with a count today are wrapped in `.face-wrap` with a badge button (`data-badge-for`, `data-count`, `data-undo-entry`) that posts the existing `/undo` with `entry` and `item`, so the row is redrawn with the lower count (no badge at 0). Because the badge carries the entry id, a double-tapped badge takes away only one (the second gets `unknown_entry`). `/undo` and the live `/board` refetch needed no changes.
- **Tests:** `tests/good_deeds.rs`, 16 HTTP-seam tests (helpers are local to the file, `tests/common` is unchanged). They cover two earns more than 2 s apart (badge 2, two today-list entries), the refusal at 400 ms and at exactly 2 s, acceptance at 2.001 s, the window counting from the latest earn, per-person and per-deed guards, badge undo of the most recent entry, the badge vanishing at 0, a double-tapped badge, undo from the today list and toast, badges resetting at Rome local midnight, `/board` showing badges, the 404 for chores, and cross-site POST. 54 HTTP-seam tests in total, plus 6 unit tests. Mutation checks: disabling the guard fails 2 tests, `<` instead of `<=` fails 1, checking the oldest earn instead of the latest fails 1, and badging the oldest entry instead of the latest fails 1.
- **For later tickets:** `Refusal` gained a variant, so ticket 06's new refusals sit next to it. "Something else" (ticket 07) good deeds kept on the board will show badges automatically, because they are `good_deed` items. One-off entries have no item, so they get no badge and no repeat guard.
