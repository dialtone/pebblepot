# 10: Edit mode for rewards and family members

**What to build:** The Spend sheet gets its own ✏️ Edit toggle (rename, re-picture, re-price, reorder, retire rewards) and a "+ New reward" tile (name, picture, price starting at 1). In board edit mode, tapping a pot lets you rename, re-face, re-colour or retire that family member, and there's a way to add a new one (kid or parent). Retired rewards and people leave the board and Spend, and their log entries stay readable.

**Blocked by:** 07, 09

**Status:** resolved

Spec: [Pebblepot v1](../../pebblepot-v1/spec.md)

- [x] Rewards can be created, edited, reordered and retired from the Spend sheet's edit mode, and changes show in Spend.
- [x] A new family member gets a pot card and a face on every row. Renaming, re-facing and re-colouring show everywhere.
- [x] Retiring a family member removes their pot card and faces. Their log entries are kept.
- [x] Retiring or re-pricing a reward that is someone's active goal doesn't change that goal's stored price or the pot.
- [x] Covered by HTTP-seam tests.

## Comments

**2026-10-16, implemented** (PR build/ticket-10-edit-rewards-and-family):
- **No migration.** `family_member` already had `colour`, `position` and `retired`, and `item` already covered rewards.
- **Rules.**
  - Reward prices have their own range, `REWARD_PRICES`, **1 to 1000** (refusal `bad_price`, "Pick a price from 1 to 1000 pebbles"). Chores and good deeds keep 1 to 100. One `checked()` helper in `rules.rs` picks the range and the default picture by kind, and `edit_item` uses it. A blank reward picture becomes 🎁.
  - New `rules::create_item(store, ItemChange)` adds an item at the end of its kind (`MAX(position) + 1`). ＋ New reward uses it, starting at `NEW_REWARD_PRICE` = 1.
  - Rewards reuse ticket 09's `edit_item`, `retire_item` and `reorder_items`. None of them touch `log_entry` or `goal`.
  - Family members: `create_member(store, Role, MemberChange)`, `edit_member` and `retire_member`, each in one immediate transaction. A blank name is refused (`name_needed`). The colour must be `#rrggbb` (`bad_colour`; it goes into a `style` attribute). A blank face becomes 🙂. A new member goes at the end. Retiring sets `retired = 1`, so their log entries and any goal stay where they are.
  - New refusals: `BadPrice` and `BadColour`. `Role::from_key` was added.
- **Web: rewards.**
  - The Spend sheet's ✏️ Edit is `GET /spend?edit=1[&member=]` (`data-spend-edit-toggle`). Edit mode shows every reward tile with a ⠿ handle, ✏️ (`data-edit-reward`) and 🗑 (`data-retire-reward`, with `hx-confirm`), plus a ＋ New reward tile (`data-new-reward`). Tiles can't be redeemed in edit mode. ✓ Done (`data-spend-edit-done`) goes back to that person's Spend sheet, or to "whose pebbles?".
  - Routes, all answering 404 for anything that isn't an active reward:
    - `GET/POST /reward/edit` (`item`, `name`, `picture`, `price`)
    - `GET/POST /reward/new`
    - `POST /reward/retire`
    - `POST /reward/reorder` (`order`)
  - Ticket 09's `/item/*` routes still refuse rewards.
  - A reward's ✏️ or ＋ sheet opens in a new `#sheet-over` layer above the Spend sheet, which stays open underneath. That way nothing has to remember whose sheet to go back to. Every reply sends the tiles (`#reward-tiles`) out of band, so the sheet keeps its scroll. A successful save also empties `#sheet-over`. A refused save is only the toast.
  - The live refresh sends `[data-spend-edit]` and gets the tiles redrawn. `reorder.js` is now loaded on every board and works on the tiles unchanged.
- **Web: family.**
  - `pots.html` always renders a ✏️ overlay button on every pot (`data-edit-member`, which opens `GET /member/edit?member=`) and a `＋ Add someone` button (`data-new-member`, which opens `GET /member/new`). CSS shows them only under `.board.editing`. This way the pots that six different replies swap in work on any device, and no editing flag has to be threaded through. The pebble pot card is now a `div.pot` around a `button.pot-open`, like the goal pot.
  - The sheet has face, name, a palette of 10 colour swatches (`data-colour-choice`) and 🗑 Retire (`data-retire-member`, with `hx-confirm`). When adding someone it also has 🧒 Kid / 🧑 Parent (`data-role-choice`, kid by default).
  - Routes, answering 404 for anyone not on the board:
    - `POST /member/edit` (`member`, `name`, `face`, `colour`)
    - `POST /member/new` (`role`; an unknown role is a 404)
    - `POST /member/retire`
  - The reply is the pots out of band, an empty `#sheet` and a toast. Other devices redraw their faces on the live refresh.
- **Resolved ambiguities.**
  1. Reward price cap: 1 to 1000.
  2. A member's role is chosen only when adding them. The ticket lists rename, re-face, re-colour and retire, so role can't be changed later.
  3. Family members can't be reordered. The ticket doesn't ask for it, and new people go at the end.
  4. A new member's colour defaults to the first palette colour nobody on the board is using.
  5. Retiring the last family member is allowed: ＋ Add someone is still there.
  6. Someone retired with an active goal keeps that goal row untouched. It just isn't shown.
  7. A same-day undo of a retired member's entry still works, as for retired items.
  8. A reward retired while it is someone's goal leaves the goal active at its stored price and picture: Ready, Got it and Give up (refunding the stored price) all still work.
- **Tests:** 33 new HTTP-seam tests: `tests/edit_rewards.rs` (21) and `tests/edit_family.rs` (12). 180 in total, up from 147.
  - Rewards: the toggle and Done, the new reward sheet (starts at 1, 🎁), adding, renaming, re-picturing and re-pricing, the 1 to 1000 range, a blank name, past redeems and pots unchanged, reorder persisting across a restart, refused drops snapping back, retire (gone from Spend, refused for redeem and goal, its entries kept and undoable), re-pricing and retiring a goal reward (pot, saved, to go and the refund unchanged, then Ready and Got it), chores and unknown ids → 404, the live refresh tiles, and cross-site POSTs.
  - Family: the pot ✏️ and ＋ Add someone, the prefilled sheets, someone new getting a pot and a face on every row (including Spend and Something else, and after a restart), renaming, re-facing and re-colouring showing on pots, rows, ✓s, Spend, Something else, the today list and toasts, refusals, retire removing the pot and faces, a retired member refused everywhere, their log entry still there, the SSE `changed` event, and cross-site POSTs.
- **Mutation checks:** each of these fails at least one test:
  - an edit rewriting the goal's price
  - rewards using the 1 to 100 range
  - retiring a reward ending its goal
  - the live refresh ignoring `spend_edit`
  - a new member not going at the end
- **Checked in a browser** (headless Chrome over CDP, iPad landscape 1180×820 and iPhone 12 Pro size 390×844):
  - On the edit board: the pot ✏️ badges, the member sheet, renaming Kid 1 to Ada 🐱 in red, adding Grandma 👵 as a parent, and the normal board showing a fifth face on every row.
  - In the Spend sheet's edit mode: re-pricing Big toy to 250 through the overlay sheet, adding Kite at 1, a retire confirm, and pointer drags on both sizes that persisted.
  - The first pass showed the ✏️ badge covering the pot number, so it was moved to the card's corner.
  - Not checked in a browser: real touch input (mouse-generated pointer events only), a goal pot in edit mode (same overlay markup), and two devices live-syncing.
- **For tickets 12 and 13:** no migration and no new configuration. The placeholder family members (Kid 1, Kid 2, Mom, Dad) can now be renamed on day one (spec story 64): tap ✏️ Edit, then tap a pot.
