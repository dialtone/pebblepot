# Pebblepot v1

Label: wayfinder:map

## Destination

A buildable v1 spec for Pebblepot: an iPad-first responsive web app on the home server where the household's kids earn pebbles into their pots for chores and good deeds and redeem them for rewards. It should be ready to hand to `/to-spec` and then `/to-tickets` for implementation.

## Notes

- **Domain**: a single household with two kids (ages 4 and 6) and parents. Vocabulary lives in [GLOSSARY.md](../../GLOSSARY.md); use its terms. Every session consults the `domain-modeling` skill.
- **Settled while charting**:
  - There are no sign-ins and it runs on the honour system: anyone picks any family member with one tap, and there are no approval steps.
  - Every family member, parents included, has a pot and earns and redeems.
  - Pebbles are only ever earned or redeemed, never deducted as punishment. Pots never go below zero, except when a goal is set.
  - Redeeming takes effect immediately. Every earn and redeem goes in the log (which kid, what, how many, when).
  - Undo is same-day only and removes the entry from the log, via the Undo toast, a chore's ✓, a good deed's count badge, or the ✕ on that person's today list. Once a day is over, its log entries are permanent.
  - Chores are open to any family member, and every chore, daily or occasional, counts at most once per family member per day. Good deeds can repeat.
  - Good deeds work like chores: quick tiles plus a free-form one with a note and an amount.
  - Everything is in one responsive web app, served over plain HTTP on the LAN and WireGuard. It's designed for the shared iPad first and should still be usable on phones. There's no install, no offline mode and no HTTPS for now; it can become a PWA later.
- **Environment**: the home server is an x86-64 box running NixOS, and Docker containers are preferred. WireGuard already gives remote access, so the app doesn't need to be reachable from the public internet.
- **Standing preference**: plan, don't build. This repo has no git, so research findings go under `.scratch/pebblepot-v1/research/`.

## Decisions so far

- [Which stack should run Pebblepot on the NixOS home server?](issues/01-stack-on-home-server.md): Rust + htmx (user's choice over the researched Go + htmx), using SQLite in WAL mode and SSE "changed" nudges that trigger refetches, in a Docker container. The libraries are in the next line.
- [Which Rust libraries should Pebblepot use?](issues/05-rust-crates.md): axum 0.8 + tower-http 0.7 (`CsrfLayer`) on tokio, askama 0.16 (block partials for htmx), rusqlite 0.40 (bundled, backup) + rusqlite_migration + tokio-rusqlite with a `watch` version counter for SSE. Packaging is a plain Dockerfile (see the deploy line).
- [What do the kid-facing screens look like for a 4- and 6-year-old?](issues/03-kid-screens.md): one shared family board (variant B). Each row has a face button per family member, Mom and Dad included. Names sit under the pots, and tapping a pot opens that person's today list with ✕ Undo on each entry. Tapping a ✓ undoes a chore, all chores are once a day, a repeat tap on a good deed within 2 s is ignored, and tapping a good deed's count badge takes one away. Spend sits at the top, and the order is daily jobs, kind things, then other jobs (all shown, with search). Redeeming is instant with Undo.
- [How do parents manage the chore, good deed and reward lists?](issues/04-parent-screens.md): on the board itself, with no PIN. "✏️ Something else" on every section adds a new item and earns it in one step (Keep on the board toggle). An ✏️ Edit mode edits, reorders and retires chores, good deeds, rewards (in Spend) and family members (via their pot). Log entries keep their own copies.
- [Should a kid be able to save toward a goal reward, and how?](issues/06-saving-toward-a-goal.md): yes, in v1, for everyone. ⭐ Save for this on an unaffordable reward redeems it now and the pot goes negative. One goal at a time; while saving, other rewards come only out of pebbles already saved. The pot shows the goal filling up, then 🎉 Ready! and Got it. Give up goal refunds with a new log entry.
- [How is Pebblepot deployed, updated and backed up on the NixOS server?](issues/07-deploy-and-backup.md): an inline Hermes-style `modules/homelab/pebblepot.nix` in `dialtone/nixos`. It pins the public `dialtone/pebblepot` repo as a source input and builds its plain Dockerfile (`rust` → `debian:trixie-slim`) on dabass, tagged by commit. It runs on loopback port 8070 with data in `/persist/opt/services/pebblepot`, served at http://pebblepot.dabass via Traefik. Updates are manual. A nightly `VACUUM INTO` writes to `/mnt/storage` (30 daily, 12 monthly), the module ships a restore script, and failures go through `notify`.
- [What starting content does Pebblepot ship with?](issues/08-starting-content.md): the [starting-content.md](starting-content.md) defaults, loaded on first start. Family members are placeholders (Kid 1, Kid 2, Mom, Dad), all at 0 and renamed in the app. There are 9 daily chores (10 a day), 11 good deeds, 18 occasional chores and 17 rewards (5–100). New items start at 1 pebble.

## Not yet specified

Nothing left. The way is clear: see [spec.md](spec.md).

## Out of scope

- Deducting pebbles as punishment: the app is rewards-only by design.
- Accounts, sign-ins, and supporting more than one household.
- Parent approval of chores and redemptions: the honour system was chosen instead.
- Scheduled chores with due dates or missed days: only the daily and occasional lists.
- Charts, streaks, badges and other gamification beyond the pot, the log and goals.
- Native iOS or Android apps.
- Installing as a PWA, offline mode, and HTTPS for v1. The user found certificates too much to deal with for now. If the PWA idea comes back, [How does Pebblepot get HTTPS on the home network so the PWA installs?](issues/02-https-for-pwa-at-home.md) already holds the researched route.
- Off-box backup (for example restic to cloud storage): v1 relies on nightly copies on SnapRAID-protected disks.
