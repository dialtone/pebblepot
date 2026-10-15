# 03: Same-day undo

**What to build:** Mistakes can be fixed on the same day. Tapping a ✓ on a chore row undoes that chore for that person. Every toast has an Undo button. Tapping anyone's pot or name opens their **today list**: every entry from today, each with ✕ Undo. Undo removes the entry from the log completely. Entries from earlier days can't be undone.

**Blocked by:** 02

**Status:** ready-for-agent

Spec: [Pebblepot v1](../../pebblepot-v1/spec.md)

- [ ] Tapping a ✓ deletes today's matching earn entry. The pot drops, and the face comes back.
- [ ] The toast's Undo removes the entry it reported.
- [ ] Tapping a pot or name opens that person's today list, showing all of their entries for today, with ✕ Undo working on each.
- [ ] Undo of an entry from an earlier local day is refused, and that entry isn't offered in the today list.
- [ ] Undo is allowed on the same day even if it leaves a pot below zero (see the spec's Further Notes).
- [ ] Covered by HTTP-seam tests, including moving the clock to the next day.
