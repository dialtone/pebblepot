# 09: Edit mode for chores and good deeds

**What to build:** A ✏️ Edit toggle at the top of the board switches to edit mode. Every chore and good deed row gets:
- a pencil: rename, change picture, change pebble value, and switch between daily and occasional for chores
- drag handles to reorder within the section
- a 🗑 Retire button

Leaving edit mode returns to the normal board. Anyone can use it, with no PIN. Edits and retirements never change past log entries or pots.

**Blocked by:** 08

**Status:** resolved

Spec: [Pebblepot v1](../../pebblepot-v1/spec.md)

- [x] In edit mode, an item's name, picture, value and (for chores) daily or occasional can be changed, and the board reflects the change.
- [x] Rows can be reordered within a section, and the order persists.
- [x] Retiring an item removes it from the board. Its existing log entries keep their copied name, picture and amount.
- [x] Editing an item's value doesn't change any pot or past entry.
- [x] Covered by HTTP-seam tests.

## Comments

**2026-10-16, implemented** (PR build/ticket-09-edit-chores-and-deeds):
- **Rules.** Three new commands, each in one immediate transaction. None of them reads or writes `log_entry` or `goal`, so pots and past entries can't change. `rules::edit_item(store, item, ItemChange { name, picture, value, kind })` checks the name isn't blank (`name_needed`) and the value is in `SOMETHING_ELSE_VALUES`, 1 to 100 (`bad_value`). A blank picture becomes ✨. Only a chore can change kind (daily ⇄ occasional). A chore that switches goes to the end of its new section (`MAX(position) + 1`). Today's ✓ and the once-a-day rule follow it, because they key on the item id. `rules::retire_item(store, item)` sets `retired = 1`. `rules::reorder_items(store, kind, order)` takes the new order of some or all of a kind's active items. Those items swap among the places they already hold, the rest keep theirs, and positions are renumbered 1..n. A retired, unknown, repeated or other-kind id refuses the whole drop with `unknown_item`. All three work on any item kind, rewards included, so ticket 10 can reuse them. No new `Refusal` variants.
- **No migration:** `item.position` and `item.retired` were already there.
- **Read model.** Unchanged. `board::load` already skips retired items and orders by `position, id`.
- **Web.** Edit mode is the board at `/?edit=1`. A `✏️ Edit` link (`data-edit-toggle`) sits in the top strip after Spend and is hidden by CSS while editing. In edit mode:
  - `main.board.editing` holds a hidden `<input name="edit" data-edit-mode>`. `#live` and the Other jobs search add it to their `hx-include`, so `GET /board?edit=1` and `GET /occasional?edit=1` keep redrawing edit rows on that device. Other devices still get faces.
  - The `✏️ Something else` pills are hidden, and a fixed `✓ Done editing` pill (`data-edit-done`, href `/`) leaves edit mode.
  - `ItemView.edit_section: Option<&str>` switches `row.html` to the edit row (`data-edit-row`, `id="edit-row-<id>"`). It has a ⠿ handle (`data-drag-handle`, `data-reorder-section`, `data-reorder-path`), the ✏️ pencil (`data-edit-item`) and 🗑 (`data-retire`, with `hx-confirm`), and no faces. Its id differs from the normal row's, so a stray earn or undo reply can't swap faces back in.
  - New `section_view(board, section, q, editing)` and `find_on_board` helpers. `board_page` and `board_refresh` use them too.
  - Routes:
    - `GET /item/edit?item=` (`ITEM_EDIT_PATH`) returns the pencil sheet (`data-edit-sheet`): picture, name, the −/+ value stepper and, for chores only, ☀️ Every day / 🧺 Now and then radios (`name=section`).
    - `POST /item/edit` takes `item, name, picture, value, section?, q`.
    - `POST /item/retire` (`ITEM_RETIRE_PATH`) takes `item, q`.
    - `POST /item/reorder` (`ITEM_REORDER_PATH`) takes `section, order=<id>,<id>,…, q`.
  - All three POSTs answer with `edit_response.html`: every section's rows in edit mode, out of band (Other jobs filtered by `q`). A successful pencil save also sends an empty `#sheet` and an `edited` toast, and a retire sends a `retired` toast. A refused pencil save is only the toast, so the sheet stays open. A refused drop still redraws the rows, so they snap back.
  - An item that isn't a chore or good deed on the board (a reward, a retired item, an unknown id) answers 404 on all three item routes. An unknown section key also answers 404.
  - `static/reorder.js` (about 50 lines, loaded only in edit mode) drags with pointer events, so it works with touch on the iPad and phone and handles the 2-column landscape grid. On drop it posts the rows that are on screen, and it skips live-refresh swaps of the section while a row is being dragged.
  - The write middleware covers live sync, with no new code.
- **Resolved ambiguities.**
  1. The prototype has no edit mode. Edit mode is a URL (`/?edit=1`) rather than client state, so it survives a live refresh and a reload on that device only.
  2. Faces and Something else are hidden in edit mode, so a tap while editing never earns.
  3. Edited values use the same 1 to 100 range as Something else, and 0 is refused, since pebbles are never taken away.
  4. A good deed can't become a chore. The spec's daily-or-occasional switch is for chores only.
  5. Retire asks for confirmation (`hx-confirm`), because it is "for good" and has no undo in the UI.
  6. Reordering a searched Other jobs list only moves the rows that were shown.
  7. Retiring a kept Something else row is how ticket 08's undo leftover is cleared. This is tested.
- **Tests:** `tests/edit_mode.rs`, 26 HTTP-seam tests (helpers are local to the file, `tests/common` is unchanged). 147 tests in total (141 HTTP-seam + 6 unit), up from 121. They cover:
  - the toggle, edit rows and Done
  - live refresh and search keeping edit mode
  - the prefilled pencil sheet, and the good-deed sheet with no kind choice
  - name, picture and value changes showing on the board and in later earns
  - daily → occasional (✓ and once-a-day kept, search finds it) and occasional → daily
  - a good deed refused as a chore
  - blank name and out-of-range or garbled values, and 1 and 100 accepted
  - the ✨ default
  - edits leaving pots, yesterday's and today's entries and their pictures alone, with undo taking off the copied amount, for both an in-place edit and a switch
  - retire across the board, edit board, `/board` and search
  - retired entries keeping their name, picture and amount, and still undoable
  - retired items refused for earn, edit and retire
  - retiring a kept Something else after its undo
  - reorder persisting across a restart, a searched partial reorder, and Something else landing after reordered rows
  - refused reorders (other section, retired, repeated, garbled, unknown section)
  - rewards and unknown ids → 404, the SSE `changed` event, and cross-site POSTs
- **Mutation checks:** each of these fails exactly one test:
  - no kind guard
  - a switched chore keeping its old position
  - no value range
  - editing also rewriting the item's log entries
  - retire deleting log entries
  - reorder accepting any ids
  - reorder moving shown rows to the top
  - the refresh ignoring edit mode
- **Checked in a browser** (iPad Air landscape and iPhone 12 Pro): edit rows, a pointer drag that persisted, the pencil sheet moving Tidy toys to the end of Other jobs, the live refresh keeping edit rows, Done, and the ✏️ Edit toggle beside Spend on the phone.
- **For ticket 10 (rewards and family):**
  - `rules::edit_item`, `retire_item` and `reorder_items` already accept `ItemKind::Reward`. They don't touch `goal`, so re-pricing or retiring a goal reward only needs a test. The value range is `SOMETHING_ELSE_VALUES` (1 to 100), and Big toy is exactly 100, so consider a separate, larger range for prices (`edit_item` checks the range in one place).
  - The web item routes deliberately answer 404 for rewards (`find_on_board` only looks at the three board sections). Give the Spend sheet its own routes or extend `find_on_board`.
  - `pots.html` doesn't know about edit mode. It is included by six response templates, so instead of threading an `editing` flag, edit mode is a URL plus `main.board.editing`, and the hidden `[data-edit-mode]` input is what the live refresh sends. For pot taps in edit mode, either thread the flag through `member_views` callers or render both buttons and switch with `.board.editing` CSS, like the ✏️ Edit toggle. Remember `.goal-pot .pot-open`.
  - `edit_response.html` and `reorder.js` are generic over a `.rows` container with `[data-item]` children and a handle carrying `data-reorder-section` and `data-reorder-path`. Spend tiles can reuse the script if they carry the same markers.

