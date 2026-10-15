# What do the kid-facing screens look like for a 4- and 6-year-old?

Type: prototype
Status: resolved
Blocked by:

## Question

How should a kid (or a parent tapping on their behalf) move through Pebblepot when the 4-year-old can't read and the 6-year-old reads a little? The prototype should cover:
- the one-tap family member picker
- the daily chore tiles, including the ✓ once a chore is done for the day
- finding an occasional chore
- good deed tiles
- the pot: is it shown as a number, a pot filling with pebbles, or both?
- the reward list, with locked rewards showing "N more pebbles to go"
- the Undo toast

It should be designed for the shared iPad first, in both portrait and landscape, and still be usable on a phone.

## Assets

- Prototype: [prototypes/kid-screens-prototype/index.html](../prototypes/kid-screens-prototype/index.html). It has three variants, switched with `?variant=A|B|C`. Serve it with `python3 -m http.server 8765 -d .scratch/pebblepot-v1/prototypes/kid-screens-prototype`.

## Answer

**Variant B, the family board**, with the user's changes. See [prototypes/kid-screens-prototype](../prototypes/kid-screens-prototype/index.html?variant=B).

- **No picking step.** It's one shared board, and every chore and good deed row has a face button for each family member. Tap whoever did it.
- **Mom and Dad are on the board and earn too.** Every family member has a pot (see GLOSSARY.md). Four faces still fit on each row in iPad landscape.
- **Tapping a ✓ undoes it.** Once a daily chore is done for the day, that person's face turns into a ✓, and tapping the ✓ strikes today's entry. Every other earn or redeem shows an Undo toast at the top of the screen for a few seconds.
- **The top strip** shows everyone's pot as a few pebble dots plus the number, which the user said looks fine, and a **🎁 Spend** button on the right.
- **Page order:**
  - ☀️ Today's jobs (daily chores)
  - 💛 Kind things (good deeds, plus ✏️ "Something else" with a note and an amount)
  - 🧺 Other jobs (occasional chores, all listed by default, with a search box that filters them)
- **Spend** opens a sheet. You pick whose pot, then reward tiles appear. Rewards they can't afford are greyed out with "🔒 N more to go". Redeeming is **instant**, and the Undo toast is the safety net.
- **Rejected:**
  - A's pick-a-person-first flow, jar and tabs
  - C's step-by-step flow, celebration screen and confirm-before-spend
- **Saved for later:** C's ⭐ goal became [Should a kid be able to save toward a goal reward, and how?](06-saving-toward-a-goal.md).

### Amendment, after more feedback

- **Names on the pots.** Each pot at the top shows the person's name under their face.
- **Every chore is once per day.** Other jobs now work like today's jobs: once per family member per day, with the face turning into a ✓ and tapping the ✓ undoing it.
- **Fixing double-taps on kind things.** The problem was that a double-tap gave free pebbles, and Undo only reached the last tap. Kind things stay repeatable, because "shared nicely" can happen twice in a day, and are protected in three ways:
  1. A repeat tap on the same face and row within 2 seconds is ignored, with an "Already counted" message.
  2. Each face on a kind-thing row shows a badge with today's count for that person. **Tapping the badge takes one away**, undoing the most recent entry. It's a 32px badge with a small "−" marker, and it disappears at zero.
  3. **Tapping a pot or name opens that person's today list**: every entry from today with an ✕ Undo (or ↩︎ Put back). This means any entry can be undone, not just the latest.
- **Rejected alternative:** making kind things once per day as well. It's simpler, but it stops good deeds from repeating.

**2026-10-14, confirmed:** the user is happy with variant B as amended. The prototype stays under `prototypes/kid-screens-prototype/` as the reference: variant B is the winner, and A and C are kept for context.
