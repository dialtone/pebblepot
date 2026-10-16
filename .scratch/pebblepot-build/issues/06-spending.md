# 06: Spending

**What to build:** 🎁 Spend opens a sheet. You pick whose pebbles to spend, then see the reward tiles with their prices. Rewards that person can't afford are greyed out with "🔒 N more to go". Tapping an affordable reward redeems it immediately, and the toast has Undo. The redeem also appears in the today list. An ordinary redeem can never take a pot below zero.

**Blocked by:** 03

**Status:** resolved

Spec: [Pebblepot v1](../../pebblepot-v1/spec.md)

- [x] The Spend sheet shows every active reward for the chosen person, with locked or unlocked state and "N more to go".
- [x] Redeeming an affordable reward writes a redeem entry of −price (name and picture copied), and the pot drops.
- [x] Redeeming a reward priced above the pot is refused, and the pot is unchanged.
- [x] A redeem can be undone on the same day from the toast or the today list.
- [x] Covered by HTTP-seam tests.

## Comments

**2026-10-15, implemented** (built together with 04 on build/tickets-04-06):
- **Rules.** `rules::redeem(store, now, tz, member, item)` runs in one immediate transaction. It checks that the family member is active and the item is an active reward, then refuses with the new `Refusal::NotEnoughPebbles` (`not_enough_pebbles`) if price > pot. Otherwise it writes a `redeem` log entry of −price, with the name and picture copied. The pot is summed inside the same transaction, so two devices can't both spend the same pebbles. A pot already below zero after a same-day undo (allowed by the spec's Further Notes) can't redeem anything.
- **Read model.** New `board::rewards(store)` lists the active rewards in order. `board::load` and `Board` are unchanged.
- **Web.** `GET /spend` shows the sheet with a "Whose pebbles?" picker of every active family member and no tiles. `GET /spend?member=<id>` shows the same picker with that person highlighted, "N to spend" (`data-spendable`) and every reward tile (`data-reward`, `data-price`, `data-state="locked|unlocked"`, `data-more-to-go`). Locked tiles are greyed out with "🔒 N more to go" and aren't tappable. An unknown member answers 404. The 🎁 Spend button loads the sheet into `#sheet`. `POST /redeem` (form: `member`, `item`) uses `hx-swap="none"` and answers entirely out of band: the pots, the redrawn sheet (`id="spend-<member>"`) and a toast ("🦊 ⭐ -5") with ↩︎ Undo. A refusal answers 200 with a `data-refusal` toast. Undo reuses `POST /undo` unchanged, and the redeem shows up in the today list.
- **Live sync.** An open sheet holds a hidden `data-spend-member` input. `#live` now uses `hx-include="[data-search], [data-spend-member]"`, and `GET /board?spend=<id>` adds the sheet out of band. A redeem or undo on any device, this one included, therefore redraws an open Spend sheet. The undo reply itself doesn't redraw the sheet; the live refresh right after it does.
- **Resolved ambiguity:** prototype variant B opens Spend with the first kid already picked, but the spec and this ticket say Spend first asks whose pebbles. The sheet starts with only the picker, so a kid can't spend someone else's pebbles by accident. The picker stays on screen for switching.
- **Tests:** 17 new HTTP-seam tests in `tests/spending.rs`, 61 in total. Its helpers live in that file, and `tests/common` is untouched. They cover the picker, every seeded reward and price in order, locked and unlocked tiles with "N more to go" (including price = pot), redeem writing −price with the snapshot, refusal when price > pot leaving the pot and today list unchanged, spending to exactly zero and then being refused, a pot below zero after an undo being refused, undo from the toast and from the today list, the next-day undo refusal, non-reward items and unknown members, the live-refresh redraw, and a cross-site POST. Mutation checks: disabling the affordability guard fails 3 tests, and accepting non-reward items fails 11.
- **For ticket 07 (Goals):** the guard in `rules::redeem` is `price > pot`. With an active goal it becomes `price > saved` (saved = pot + goal price). `spend_view` in `src/web.rs` computes `spendable` (currently the pot), and locks and "N more to go" derive from it. Switch it to saved, and the `data-spendable` / "N to spend" line already exists. Locked tiles are plain `<div>`s, ready to gain "⭐ Save for this". There is no `goal` table yet: migration 0001 has only the `goal_set`/`goal_refund` entry types.
