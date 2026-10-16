# 03: Same-day undo

**What to build:** Mistakes can be fixed on the same day. Tapping a ✓ on a chore row undoes that chore for that person. Every toast has an Undo button. Tapping anyone's pot or name opens their **today list**: every entry from today, each with ✕ Undo. Undo removes the entry from the log completely. Entries from earlier days can't be undone.

**Blocked by:** 02

**Status:** resolved

Spec: [Pebblepot v1](../../pebblepot-v1/spec.md)

- [x] Tapping a ✓ deletes today's matching earn entry. The pot drops, and the face comes back.
- [x] The toast's Undo removes the entry it reported.
- [x] Tapping a pot or name opens that person's today list, showing all of their entries for today, with ✕ Undo working on each.
- [x] Undo of an entry from an earlier local day is refused, and that entry isn't offered in the today list.
- [x] Undo is allowed on the same day even if it leaves a pot below zero (see the spec's Further Notes).
- [x] Covered by HTTP-seam tests, including moving the clock to the next day.

## Comments

**2026-10-15, implementation** (merged to main in PR #1, e88210d):

- Rules: `rules::undo_entry(store, now, tz, entry_id)` deletes the log entry in one immediate transaction, only if its `local_day` is today in the household time zone. Refusals: `not_undoable` (earlier day), `unknown_entry` (already undone). No pot check, so undo may leave a pot below zero, as the spec's Further Notes allow (not reachable until redeem exists). `earn_chore` now returns the new entry id for the toast.
- Read model: `board.done` maps (member, item) to today's entry id, so each ✓ knows which entry it undoes. New `board::today_list(store, member, today)`: today's entries, most recent first.
- Web: `GET /today?member=<id>` returns the today list sheet (pot cards load it into `#sheet`). `POST /undo` takes `entry=<id>`, plus `item=<id>` from a ✓ so the row is redrawn even when the undo is refused. Every undo button uses `hx-swap="none"`, and the reply is all out of band: the row (rows now have `id="row-<item>"`), the pots, the toast and that member's today list (`id="today-<member>"`, only swapped in if it's open). Earn toasts have a "↩︎ Undo" button. Sheet closing is client-side through `hx-on:click`.
- Tests: `tests/undo.rs` (12 tests, HTTP seam only), covering undo by ✓, the toast and the today list, double undo, the next-day refusal after local midnight in Europe/Rome, a stale ✓ from yesterday, and cross-site POST. Mutation check: disabling the same-day guard fails 2 tests.
