# 09: Edit mode for chores and good deeds

**What to build:** A ✏️ Edit toggle at the top of the board switches to edit mode. Every chore and good deed row gets:
- a pencil: rename, change picture, change pebble value, and switch between daily and occasional for chores
- drag handles to reorder within the section
- a 🗑 Retire button

Leaving edit mode returns to the normal board. Anyone can use it, with no PIN. Edits and retirements never change past log entries or pots.

**Blocked by:** 08

**Status:** ready-for-agent

Spec: [Pebblepot v1](../../pebblepot-v1/spec.md)

- [ ] In edit mode, an item's name, picture, value and (for chores) daily or occasional can be changed, and the board reflects the change.
- [ ] Rows can be reordered within a section, and the order persists.
- [ ] Retiring an item removes it from the board. Its existing log entries keep their copied name, picture and amount.
- [ ] Editing an item's value doesn't change any pot or past entry.
- [ ] Covered by HTTP-seam tests.
