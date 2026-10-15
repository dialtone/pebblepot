# Should a kid be able to save toward a goal reward, and how?

Type: grilling
Status: resolved
Blocked by: 03

## Question

The kid screens prototype showed a ⭐ **goal**: a kid picks a reward they can't afford yet, and their pot shows progress toward it, for example as rows of 10 dots filling toward the price. The user liked the idea and asked to come back to it later. To settle:
- Does each kid have one goal at a time, or several?
- Who sets a goal?
- What happens to the goal when it's redeemed, or when the reward is edited or retired?
- How does a goal show on the family board, next to each kid's pot?
- Is it in v1, or does it go under out of scope next to streaks and badges?

Variant C of [prototypes/kid-screens-prototype](../prototypes/kid-screens-prototype/index.html?variant=C) has a rough version to react to.

## Answer

**Goals are in v1.** A **goal** (see GLOSSARY.md) is a reward redeemed before its owner can afford it.

- **Setting a goal.** In Spend, a reward that person can't afford yet offers **⭐ Save for this** instead of only "🔒 N more to go". Choosing it redeems the reward straight away, and the pot goes negative by the shortfall. Example: Ben has 8, chooses 🎬 Movie night (30), and his pot shows −22.
- **Only a goal can take a pot below zero.** An ordinary redeem still needs enough pebbles.
- **One goal at a time** per family member. Mom and Dad can have goals too.
- **Spending while saving.** "Saved" means pot + the goal's price (8 in the example), and it's the spending money while a goal is active. Any reward can be redeemed as long as "saved" doesn't drop below zero, so the pot never goes below minus the goal's price. Ben can buy a ⭐ Sticker (5), leaving 3 saved and his pot at −27, but not 🍦 Pick dessert (10). Spend shows "N to spend" while a goal is active.
- **Pot display during a goal.** The pot at the top shows the goal reward's picture filling from grey to full colour as "saved" grows, with "N to go" underneath.
- **Reaching it.** When the pot is back to zero or above, the goal shows in full colour with 🎉 **Ready!** until someone taps **Got it** when the reward is handed over. Then the pot goes back to showing pebbles.
- **Giving up.** **Give up goal** in Spend adds a new log entry that gives the full price back, so nothing is edited and it fits the permanent log. On the same day, ordinary Undo also works.
- **Out of scope** now lists goals as the one exception to "no gamification beyond the pot and log".

## Comments

**2026-10-15, user input (verbatim):** "Spend money towards a token goal and go negative, the negative amount is your distance to the goal I suppose, once you go positive you'll be able to acquire the goal. In that case instead of a pot with pebbles you should probably have your goal item."

**2026-10-15, grilling answers:**
- Q1: only a goal can take a pot below zero.
- Q2: one goal at a time; other rewards can be redeemed, but only from pebbles already saved.
- Q2b: confirmed the "saved can't drop below zero" rule.
- Q3: a goal is only offered for rewards that can't be afforded yet.
- Q4: as proposed (the 4-year-old reads numbers).
- Q5: yes.
- Q6: yes.
- Q7: yes.
- Q8: in v1.
