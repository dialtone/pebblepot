# Pebblepot v1

Status: ready-for-agent

Map: [map.md](map.md). Vocabulary: [GLOSSARY.md](../../GLOSSARY.md). Default data: [starting-content.md](starting-content.md). Board reference: [kid screens prototype, variant B](prototypes/kid-screens-prototype/index.html?variant=B).

## Problem Statement

A household with two young kids (aged 4 and 6) wants to reward good behaviour and chores with something the kids can see building up and later spend. Doing it on paper or from memory doesn't work. Nobody remembers who already made their bed today, a 4-year-old can't read a chart, the kids can't see how close they are to a treat, and mistakes and double-counting creep in. The family wants one shared screen in the kitchen, the iPad, that anyone can tap, kids or grown-ups. It should keep an honest record and make saving up for something visible and exciting. It has to run on their own home server with no accounts, cloud services or sign-ins.

## Solution

Pebblepot is a responsive web app, designed for the iPad first, served from the family's NixOS home server at http://pebblepot.dabass over the LAN and WireGuard. It is one shared **family board**:

- **The top strip** shows every family member's **pot**: their face, their name underneath, a few pebble dots and the number. A **🎁 Spend** button sits at the right. A pot that is saving for a **goal** shows the goal reward's picture filling up instead, with "N to go".
- **☀️ Today's jobs:** the daily chores. Each row has a face button per family member, and you tap whoever did it. The face turns into a ✓, and tapping the ✓ undoes it.
- **💛 Kind things:** the good deeds, with the same face buttons. They can repeat. A badge on each face shows today's count, and tapping the badge takes one away.
- **🧺 Other jobs:** the occasional chores, all listed and filterable with a search box. They behave like today's jobs.
- **✏️ Something else** on each section adds a new chore or good deed and gives the pebbles in one step.
- **✏️ Edit mode** lets anyone edit, reorder or retire chores, good deeds, rewards and family members.
- **Tapping a pot** opens that person's **today list**, where any of today's entries can be undone.
- **Spend** shows the rewards. Redeeming is instant. A reward someone can't afford offers **⭐ Save for this**, which redeems it now and lets their pot go negative until they've earned it back.
- **Every device stays in sync.** A tap on one phone shows up on the iPad straight away.

There are no sign-ins, PINs or approvals: the family runs on the honour system, and the log keeps everyone honest. Once a day is over, its log is permanent.

## User Stories

### The board
1. As a family member, I want to open Pebblepot at http://pebblepot.dabass on the iPad or my phone, so that I can use it without installing anything or signing in.
2. As a family member, I want one shared board with no "who are you?" step, so that recording something takes one tap.
3. As a kid who can't read well yet, I want every chore, good deed and reward to have a big picture, so that I can find things without reading.
4. As a kid, I want each item's pebble value shown as dots, so that I can count how many I'll get.
5. As a family member, I want to see every pot at the top with each person's face and name, so that we all know where everyone stands.
6. As a family member, I want the board laid out for the iPad in portrait and landscape, so that it's comfortable on the kitchen tablet.
7. As a parent, I want the board to work on my phone too, so that I can record something from another room or away from home over WireGuard.
8. As a family member, I want the sections in the order Today's jobs, Kind things, Other jobs, so that the things we do most are at the top.

### Earning
9. As a kid, I want to tap my face on a chore's row when I've done it, so that I earn its pebbles straight away.
10. As a parent, I want to tap my child's face on their behalf, so that the 4-year-old gets credit even when I'm the one holding the iPad.
11. As a parent, I want Mom and Dad to have faces on every row and their own pots, so that the kids see the grown-ups pitching in.
12. As a family member, I want each chore to count at most once per person per day, so that nobody can make their bed five times for extra pebbles.
13. As a family member, I want a chore I've done today to show a ✓ instead of my face, so that I can see what's left.
14. As a family member, I want the ✓s to reset at local midnight, so that tomorrow starts fresh.
15. As a family member, I want occasional chores ("Other jobs") to work the same way as daily ones, once per person per day with a ✓, so that the board behaves consistently.
16. As a family member, I want all occasional chores listed by default, so that I can browse them without searching.
17. As a family member, I want to type in a search box to filter the occasional chores, so that I can find "rake leaves" quickly in a long list.
18. As a parent, I want to tap a kid's face on a good deed's row whenever I catch them being kind, so that good behaviour is rewarded on the spot.
19. As a parent, I want good deeds to be repeatable in a day, so that sharing nicely twice counts twice.
20. As a family member, I want a quick second tap on the same face and the same good deed within 2 seconds to be ignored, with an "Already counted" message, so that a double-tap doesn't give free pebbles.
21. As a family member, I want each face on a good deed's row to show how many times that person got it today, so that a mistake is easy to spot.
22. As a family member, I want a short toast after every earn showing who got what (for example "🐻 🛏️ +2") with an Undo button, so that I can confirm it worked and fix a mistake at once.

### Something else (adding new things on the fly)
23. As a family member, I want a "✏️ Something else" button on Today's jobs, Other jobs and Kind things, so that I can reward something that isn't on the list yet.
24. As a family member, I want "Something else" to ask for a name, a picture, a pebble value and who did it, so that the new item and the pebbles are recorded in one step.
25. As a family member, I want a new item's pebble value to start at 1, so that a quick add still earns something.
26. As a family member, I want a "Keep on the board" toggle that starts on, so that a new item becomes a permanent tile unless I mark it as a one-off.
27. As a family member, I want a one-off "Something else" to appear only in the log, so that rare events don't clutter the board.
28. As a family member, I want something added under Today's jobs to become a daily chore and something added under Other jobs to become an occasional chore, so that it lands in the right section.

### Undo and fixing mistakes
29. As a family member, I want to tap a ✓ to undo that chore for that person, so that a wrong tap is easy to reverse.
30. As a family member, I want to tap a good deed's count badge to take one away, removing the most recent, so that I can fix a double-count right on the row.
31. As a family member, I want to tap anyone's pot or name to open their today list, with an ✕ Undo beside every entry from today, so that I can fix any mistake from today, not just the last one.
32. As a family member, I want undo to remove the entry from the log completely, so that the record only shows what really happened.
33. As a parent, I want undo to work only on entries from the same day, so that the past can't be rewritten.
34. As a parent, I want past days' log entries to be permanent, so that pots stay trustworthy.

### Spending
35. As a family member, I want a 🎁 Spend button at the top of the board, so that spending pebbles is always one tap away.
36. As a family member, I want Spend to ask whose pebbles are being spent and then show the reward tiles with their prices, so that I can choose a reward.
37. As a family member, I want rewards someone can't afford to look locked, with "🔒 N more to go", so that kids know how far they have to go.
38. As a family member, I want redeeming an affordable reward to happen immediately, without a confirmation step, so that it's quick.
39. As a family member, I want an Undo toast after redeeming, so that an accidental purchase can be reversed.
40. As a family member, I want a pot never to go below zero from an ordinary redeem, so that you can only spend what you've earned.
41. As a parent, I want a few grown-up rewards (for example Sleep in, Breakfast in bed) on the shared list, so that Mom and Dad have something to spend on too.

### Goals
42. As a kid, I want a reward I can't afford to offer "⭐ Save for this", so that I can commit to something bigger.
43. As a kid, I want setting a goal to redeem the reward now and take my pot below zero by the shortfall, so that the negative number shows how far I am from it.
44. As a family member, I want only one goal at a time per person, so that saving stays simple.
45. As a kid, I want my pot at the top to show my goal's picture filling up from grey to colour, with "N to go", so that I can see my progress.
46. As a kid saving for a goal, I want to still spend on small rewards using pebbles I've already saved, so that saving doesn't mean no treats at all.
47. As a parent, I want spending while saving to be refused if it would use pebbles not yet earned (saved can never drop below zero), so that the goal is never pushed further away than its price.
48. As a family member, I want Spend to show "N to spend" while a goal is active, so that it's clear how much can be spent.
49. As a kid, I want my goal to show 🎉 Ready! when my pot is back to zero or more, so that I know I've earned it.
50. As a parent, I want a Got it button on a Ready goal, so that I mark it as handed over and the pot goes back to showing pebbles.
51. As a family member, I want to be able to give up a goal, so that a kid who changes their mind gets the full price back.
52. As a parent, I want giving up a goal to add a new refund entry rather than change history, so that the log stays permanent.
53. As a parent, I want Mom and Dad to be able to set goals too, so that everyone plays by the same rules.

### Edit mode and lists
54. As a parent, I want a ✏️ Edit toggle at the top of the board, so that I can manage the lists without a separate admin area.
55. As a family member in edit mode, I want a pencil on every row to change its name, picture, pebble value, and whether it's daily or occasional, so that the lists stay right for our family.
56. As a family member in edit mode, I want drag handles to reorder rows within a section, so that the most common things sit at the top.
57. As a family member in edit mode, I want a 🗑 Retire button on every row, so that items we no longer use leave the board.
58. As a family member, I want edits and retirements never to change past log entries or pots, so that history stays accurate.
59. As a family member, I want the Spend sheet to have its own Edit toggle and a "+ New reward" tile (name, picture, price), so that rewards are managed the same way.
60. As a family member in edit mode, I want to tap a pot to add, rename, re-face, re-colour or retire a family member, so that the people on the board are managed the same way.
61. As a parent, I want anyone to be able to use edit mode, with no PIN, so that the honour system stays simple.

### Live sync
62. As a family member, I want a tap on any device to show up on every other open device within a moment, so that the iPad and our phones always agree.
63. As a family member, I want the board to refresh itself when the iPad wakes from sleep or reconnects, so that it never shows stale pots.

### First start and data
64. As a parent, I want Pebblepot to start with placeholder family members (Kid 1, Kid 2, Mom, Dad), all at 0 pebbles, that I rename in edit mode, so that we can use it straight away and our real names never live in the code.
65. As a parent, I want it to start with sensible default chores, good deeds and rewards (see starting-content.md), so that we don't begin from a blank board.
66. As a parent, I want the default content loaded only once, on first start, so that my edits are never overwritten.

### Running it at home
67. As the household's admin, I want Pebblepot to run as a Docker container defined in a module inside my nixos repo, in the same style as Hermes, so that it fits how I run everything else on dabass.
68. As the admin, I want the container image built on dabass from a plain Dockerfile in the public Pebblepot repo, pinned by my flake.lock, so that there's no CI or registry to look after.
69. As the admin, I want to update by running `nix flake update pebblepot` and `nixos-rebuild switch`, so that upgrades happen only when I choose.
70. As the admin, I want the app published only on loopback and reached through Traefik with my LAN whitelist, so that it's not exposed beyond the home network and WireGuard.
71. As the admin, I want a nightly consistent backup of the database onto my SnapRAID-protected storage, keeping 30 daily and 12 monthly copies, so that a failed disk doesn't lose the kids' pebbles.
72. As the admin, I want a restore script that stops the app, puts a chosen backup in place and starts the app again, so that recovering is one command.
73. As the admin, I want a failure of the app or of the backup to send an alert through my existing `notify` script, so that I find out without checking.
74. As the admin, I want stopping the container to be quick, so that `nixos-rebuild` and restarts don't hang on open live-update connections.

### Local development
75. As the developer, I want `cargo run` with no setup to start Pebblepot locally with sensible defaults (data in `./data`, port 8070, the system time zone, listening on localhost), so that I can work on it on my laptop without Nix or Docker.
76. As the developer, I want to set the listen address to `0.0.0.0` and open `http://<my-mac-ip>:8070` on the iPad, so that I can try changes on the real device before deploying.
77. As the developer, I want to build and run the production Dockerfile on my Apple Silicon Mac with Docker Desktop (port and data folder mapped), so that I can check the image before updating dabass.

## Implementation Decisions

### Architecture and stack
- **One Rust binary** serves server-rendered HTML with htmx 2 (a single vendored file plus the SSE extension, with no npm and no front-end build step).
- **Crates** (see [Which Rust libraries should Pebblepot use?](issues/05-rust-crates.md)):
  - axum 0.8 on tokio
  - tower-http 0.7 with `CsrfLayer` for cross-origin protection, which works over plain HTTP by comparing the Origin and Host headers
  - askama 0.16 templates, with block partials for htmx swaps
  - rusqlite 0.40 with bundled SQLite, plus rusqlite_migration and tokio-rusqlite: a single connection on its own thread, in WAL mode
  - a tokio `watch` version counter for fan-out
  - jiff for local-day calculation
- **Plain HTTP only.** No HTTPS, PWA manifest, service worker or offline mode.
- **Configuration through environment variables.** Every setting has a default, so `cargo run` works with no setup on the developer's Mac:
  - listen address: default `127.0.0.1`; `0.0.0.0` exposes it on the LAN, for example to try it on the real iPad
  - listen port: default 8070
  - data directory: default `./data`, created if missing
  - household time zone: an IANA name, defaulting to the system time zone

  The NixOS module sets all four explicitly: `0.0.0.0` inside the container, 8070, the mounted data directory, and the household's time zone.

### Modules
1. **Domain core (pebble rules).** A deep module that owns every rule in this spec and exposes a small command interface: *earn chore*, *earn good deed*, *earn something else* (with or without keeping it on the board), *redeem*, *set goal*, *give up goal*, *mark goal handed over*, *undo entry*, and the edit-mode commands (*create, update, reorder, retire* for chores, good deeds, rewards and family members). It also has a read model, *board state for today*: every pot, each person's goal state and saved amount, per-person ✓ state for chores, per-person good-deed counts, and each person's today list. Every command runs in a single immediate SQLite transaction, and a refusal comes back as a typed reason (already done today, repeat tap, not enough pebbles, goal already active, not undoable).
2. **Store.** SQLite schema, migrations, and the transactional queries the domain core uses.
3. **Clock.** Supplies "now" and the household time zone, and so the local day. Production uses system time. Tests inject a controllable clock that can be moved forward.
4. **Web layer.** Axum routes for the board page, the htmx partials (top strip, each section, Spend sheet, today list, edit forms) and the command endpoints. Commands are form POSTs that answer with re-rendered partials. CSRF protection comes from `CsrfLayer`.
5. **Live updates.** One SSE endpoint. After any committed write the version counter is bumped, and every open stream receives a `changed` event whose data is the new version number. It has to be non-empty, because SSE drops events with empty data. Clients refetch the board when they get a `changed` event, when the page becomes visible again, and after reconnecting. If the stream errors, for example during a restart, the client re-opens it, because the browser won't retry after an error response. A keep-alive is sent every 15 seconds. On shutdown a cancellation token closes all streams at once.
6. **Seed.** On start-up, if the database has no family members, it loads the content in [starting-content.md](starting-content.md): placeholder family members Kid 1 and Kid 2 (kids), Mom and Dad (parents), renamed in edit mode on day one, all chores, good deeds and rewards with their pictures, values and order. This never runs again.
7. **Packaging.** A two-stage Dockerfile in the Pebblepot repo: compile in the official `rust` image, then copy the binary plus templates, migrations and static assets into `debian:trixie-slim`.
8. **NixOS module** (lives in the `dialtone/nixos` repo as a homelab module in the same style as Hermes):
   - **Source and build:** pins the public `dialtone/pebblepot` repo as a source-only flake input. A oneshot build service runs before the container and builds the Dockerfile into the local image tag `pebblepot:<shortRev>`, skipping the build if the tag already exists.
   - **Container:** an oci-containers container on the Docker backend, using that local image and never pulling. It publishes `127.0.0.1:8070` and keeps its data under the homelab service-config root (`/persist/opt/services/pebblepot`, local disk).
   - **Proxy:** a Traefik `apps` entry giving `pebblepot.dabass` behind `local-ip-whitelist`.
   - **Backup:** a nightly systemd timer runs SQLite `VACUUM INTO` to write a timestamped copy into a Pebblepot folder on the SnapRAID-protected main array, then prunes to 30 daily and 12 monthly copies. It never copies the raw `.db`.
   - **Restore command** (`pebblepot-restore <backup-file>`): stops the container, replaces the database, deletes any `-wal`/`-shm` files and starts the container again.
   - **Alerts:** `OnFailure=` on the container and backup units calls the existing `notify` script.

### Data model (schema)
- **family_member:** id, name, face (emoji), colour, role (kid or parent), position, retired flag.
- **item:** id, kind (daily chore, occasional chore, good deed, reward), name, picture (emoji), value (pebbles earned, or price for rewards), position within its kind, retired flag.
- **log_entry:**
  - id and family member
  - entry type: earn, redeem, goal set, goal refund
  - item id, nullable for one-off "something else" entries
  - **snapshot copies** of the item's name and picture
  - signed amount, optional note
  - the **local day** it belongs to, and a created-at timestamp
- **goal:** id, family member, the goal's own log entry (type "goal set"), reward snapshot (name, picture, price), and status: active, handed over or given up. "Ready" is not stored. It is *active with pot ≥ 0*.
- **A pot is the sum of that person's log entries.** It is never stored separately.

### Rules (all enforced in the domain core)
- **Chores (daily and occasional):** at most one earn per family member per chore per local day. The local day comes from the clock and the household time zone, and resets at local midnight.
- **Good deeds:** repeatable, but an identical earn (same person, same good deed) within 2 seconds of the previous one is refused as a repeat tap.
- **Something else:** creates the item (in the chosen section, value defaulting to 1) if "keep on the board" is on, then earns it. If it's off, it records a one-off log entry with no item.
- **Undo:**
  - deletes a log entry, and is allowed only if the entry's local day is today
  - a chore's ✓ and the today list's ✕ undo that specific entry
  - a good deed's badge undoes that person's most recent entry for that good deed today
  - undoing a "goal set" entry also deletes its goal
  - undo is always allowed on the same day, even if it leaves a pot below the usual limits; the board simply shows the result
- **Redeem without an active goal:** allowed only if price ≤ pot.
- **Saved and spending during a goal:** with an active goal, *saved* = pot + goal price. Redeeming is allowed only if price ≤ saved. Spend shows saved as "N to spend".
- **Set goal:**
  - allowed only for a reward priced above the person's pot, and only if they have no active goal
  - writes a "goal set" log entry of −price and creates the goal
- **Give up goal:** writes a "goal refund" log entry of +price and marks the goal given up.
- **Got it:** allowed only when the goal is active and the pot ≥ 0. Marks it handed over, and the pot display goes back to pebbles.
- **Edits never touch log entries.** Retired items and family members disappear from the board and from Spend, and their log entries stay readable.
- **No access control anywhere** (honour system).

### Board details (from the accepted prototype)
- Top strip: one pot card per active family member (face, name under it, a few pebble dots, the number) plus the Spend button. During a goal, the card shows the goal picture with a grey-to-colour fill in proportion to saved ÷ price, and "N to go".
- Rows: picture, name, value dots, then one face button per active family member, in family-member order.
- Toasts appear at the top of the screen, last about 5 seconds and have an Undo button.

## Testing Decisions

- **One test seam: the HTTP app.** Tests build the full axum app in-process with a fresh temporary SQLite file and an injected controllable clock. They send the same form POSTs and GETs the board's htmx sends and check what comes back. They never call store queries or domain internals directly, and never check the database.
- **What makes a good test:** it describes behaviour a family member would notice, such as "Kid 2 taps Make bed twice; second tap refused; pot +1 once", and checks observable output: pot numbers, ✓ state, badge counts, locked or unlocked rewards, "N to go" or "N to spend", Ready state, today-list contents and refusal messages. Templates should expose stable semantic markers (for example data attributes for person, item, state and pot value) so tests don't depend on layout or wording.
- **Tests must cover:**
  - the once-per-day limit for both chore kinds, and the reset after moving the clock past local midnight
  - the good-deed repeat-tap guard, inside and outside 2 seconds
  - undo by ✓, by badge (most recent first) and from the today list, and that undo is refused for yesterday's entries
  - redeem allowed and refused
  - set goal: only when unaffordable, only one at a time, the pot going negative
  - spending while saving, allowed and refused at the saved boundary
  - Ready, then Got it
  - give up goal: refund entry, pot restored
  - undoing a goal-set entry on the same day
  - Something else with "keep on the board" on and off, with the default value of 1
  - edit and retire never changing pots or log snapshots
  - the first-start seed loading once and not reloading on restart
  - live updates: an open SSE stream receives a `changed` event with non-empty data after a command
  - CSRF: a cross-origin POST is rejected
- **Prior art:** none, as the repo is new. The prototype's in-memory state model and its seeded scenarios are a useful source of test scenarios.
- **Not automated:**
  - backup and restore, verified by hand once at go-live with the restore script
  - the NixOS module, verified by `nixos-rebuild` on dabass
  - visual layout, settled by the prototype

## Out of Scope

- Deducting pebbles as punishment.
- Accounts, sign-ins, PINs, roles with different permissions, and more than one household.
- Parent approval of chores or redemptions.
- Scheduled chores with due dates or missed days.
- Charts, streaks, badges and other gamification, apart from the pot, the log and goals.
- Native iOS or Android apps.
- Installing as a PWA, offline mode, and HTTPS. The researched route is in [How does Pebblepot get HTTPS on the home network so the PWA installs?](issues/02-https-for-pwa-at-home.md) if this is revisited.
- Fixing log entries from earlier days, and any history browsing beyond today's list.
- Off-box backup (for example restic to cloud storage).
- Per-person reward lists. Every reward is on one shared list.

## Further Notes

- **Decision trail:** every decision here traces back to a ticket on the [map](map.md): stack, libraries, board, lists and edit mode, goals, deployment and starting content.
- **Undo can break the usual limits.** Undo is allowed on the same day even if it pushes a pot below zero, or "saved" below zero during a goal. Refusing a correction felt worse than briefly showing an odd balance. This was settled in writing the spec rather than asked directly, so revisit it if it feels wrong in use.
- **Building the image on dabass** compiles Rust on the first start after each update. That takes a few minutes and downloads crates, and was accepted to avoid CI and a registry.
- **iPad live updates:** the refetch on visibility change and the reconnect after a stream error are not optional. The browser's SSE client stops retrying for good after an error response, for example while the container restarts.
- **Local development:** the binary needs no system libraries (SQLite is bundled, htmx is vendored), and both base images (`rust`, `debian:trixie-slim`) are multi-architecture, so `cargo run` and `docker build` both work natively on an Apple Silicon Mac. The CSRF check compares the Origin and Host headers, so it works with a bare LAN IP with no configuration.
- **Library versions:** axum is expected to have one breaking upgrade to 0.9, and askama and rusqlite release breaking 0.x versions occasionally. All are small upgrades.
