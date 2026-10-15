# 06: Spending

**What to build:** 🎁 Spend opens a sheet. You pick whose pebbles to spend, then see the reward tiles with their prices. Rewards that person can't afford are greyed out with "🔒 N more to go". Tapping an affordable reward redeems it immediately, and the toast has Undo. The redeem also appears in the today list. An ordinary redeem can never take a pot below zero.

**Blocked by:** 03

**Status:** ready-for-agent

Spec: [Pebblepot v1](../../pebblepot-v1/spec.md)

- [ ] The Spend sheet shows every active reward for the chosen person, with locked or unlocked state and "N more to go".
- [ ] Redeeming an affordable reward writes a redeem entry of −price (name and picture copied), and the pot drops.
- [ ] Redeeming a reward priced above the pot is refused, and the pot is unchanged.
- [ ] A redeem can be undone on the same day from the toast or the today list.
- [ ] Covered by HTTP-seam tests.
