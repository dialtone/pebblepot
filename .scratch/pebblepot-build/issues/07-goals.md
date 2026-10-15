# 07: Goals

**What to build:** A family member can save toward a reward they can't afford yet:
- **Setting a goal.** A locked reward in Spend offers ⭐ Save for this. Choosing it redeems the reward now as a goal, and the pot goes negative by the shortfall. Only one goal per person at a time.
- **Pot display.** During a goal, that person's pot card shows the reward's picture filling from grey to colour, in proportion to "saved" (pot + price) ÷ price, with "N to go".
- **Spending while saving.** Other rewards can be redeemed only from what's saved: Spend shows "N to spend", and a redeem is refused if price > saved.
- **Reaching it.** When the pot is back to zero or above, the card shows 🎉 Ready! with a Got it button. Got it marks the goal handed over, and the card goes back to pebbles.
- **Giving up.** Give up goal adds a refund entry of +price. On the same day, undoing the goal-set entry removes the goal entirely.

**Blocked by:** 06

**Status:** ready-for-agent

Spec: [Pebblepot v1](../../pebblepot-v1/spec.md)

- [ ] ⭐ Save for this is offered only for rewards priced above the pot, and only when the person has no active goal. It writes a goal-set entry of −price and creates the goal.
- [ ] The pot card shows the goal picture with a fill proportional to saved ÷ price, plus "N to go".
- [ ] With an active goal, Spend shows "N to spend" = saved. A redeem with price ≤ saved succeeds; price > saved is refused.
- [ ] When the pot is ≥ 0, the card shows Ready! with Got it. Got it marks the goal handed over and the card shows pebbles again.
- [ ] Give up goal writes a refund entry of +price, marks the goal given up, and restores the pot.
- [ ] Undoing the goal-set entry on the same day deletes the goal.
- [ ] Mom and Dad can set goals the same way.
- [ ] Covered by HTTP-seam tests (example from the spec: pot 8, goal 30 → −22, saved 8; Sticker 5 allowed, then Dessert 10 refused).
