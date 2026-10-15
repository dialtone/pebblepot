# 08: ✏️ Something else

**What to build:** Each section (Today's jobs, Kind things, Other jobs) has a ✏️ Something else button. It asks for a name, a picture, a pebble value (starting at 1) and who did it, then earns it in one step. A **Keep on the board** toggle starts on:
- **On:** the new item is created in that section, as a daily chore, good deed or occasional chore.
- **Off:** it's a one-off log entry with no item.

**Blocked by:** 04

**Status:** ready-for-agent

Spec: [Pebblepot v1](../../pebblepot-v1/spec.md)

- [ ] Something else on each section creates the right kind of item when Keep on the board is on, and the new row appears on the board.
- [ ] With Keep on the board off, only a log entry is written (with the given name, picture and value), and no row appears.
- [ ] The pebble value defaults to 1.
- [ ] The earn shows in the pot, the toast and the today list, and can be undone on the same day.
- [ ] Covered by HTTP-seam tests.
