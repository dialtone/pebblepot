# 10: Edit mode for rewards and family members

**What to build:** The Spend sheet gets its own ✏️ Edit toggle (rename, re-picture, re-price, reorder, retire rewards) and a "+ New reward" tile (name, picture, price starting at 1). In board edit mode, tapping a pot lets you rename, re-face, re-colour or retire that family member, and there's a way to add a new one (kid or parent). Retired rewards and people leave the board and Spend, and their log entries stay readable.

**Blocked by:** 07, 09

**Status:** ready-for-agent

Spec: [Pebblepot v1](../../pebblepot-v1/spec.md)

- [ ] Rewards can be created, edited, reordered and retired from the Spend sheet's edit mode, and changes show in Spend.
- [ ] A new family member gets a pot card and a face on every row. Renaming, re-facing and re-colouring show everywhere.
- [ ] Retiring a family member removes their pot card and faces. Their log entries are kept.
- [ ] Retiring or re-pricing a reward that is someone's active goal doesn't change that goal's stored price or the pot.
- [ ] Covered by HTTP-seam tests.
