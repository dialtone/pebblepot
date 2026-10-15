# Which stack should run Pebblepot on the NixOS home server?

Type: research
Status: resolved
Blocked by:

## Question

What server runtime, database and PWA framework should Pebblepot use, running as a Docker container on an x86-64 NixOS home server? The stack has to:
- serve an installable PWA to phones and a shared tablet
- keep every device's view of the pots and log current when another device taps (decide between polling and push such as SSE or WebSockets)
- store a small single-household log reliably, for example in SQLite on a volume
- be cheap to build and maintain for one developer

Recommend one stack, give the runner-up and the reasons, and note the NixOS/Docker packaging facts the deployment fog will need.

## Previous answer (superseded 2026-10-07, being re-researched)

**Recommendation: SvelteKit 3 (`@sveltejs/adapter-node`) on Node.js 24 LTS, with SQLite through Node's built-in `node:sqlite` in WAL mode on a host volume, Server-Sent Events for live updates, and SvelteKit's built-in service worker plus a web manifest for the PWA. Ship it as one Docker image.** A single TypeScript codebase holds the kid screens, the parent screens, the API and the live-update stream, which is the cheapest thing for one developer to build and maintain. `node:sqlite` has been a release candidate since v24.15.0 and needs no flag. That means no native addon and no compiler in the image. SSE fits because updates only flow one way: taps are ordinary POSTs, and after any log write the server sends a small "changed" event. Each device then refetches the pots and log, and also refetches when it wakes or reconnects. `EventSource` reconnects on its own. The HTTP/1.x limit of 6 connections per domain is fine at one tab per device. Polling would work but makes taps feel laggy on other devices, and WebSockets add a two-way protocol Pebblepot doesn't need.

**Runner-up: PocketBase** (one Go binary with embedded SQLite, SSE realtime, an admin dashboard and built-in backups) behind a static PWA. It came second because its docs warn there is no backward-compatibility guarantee before v1.0. Also, Pebblepot's rules would be split between PocketBase hooks and API rules and a separate front end: pots never below zero, the once-per-kid-per-day daily-chore limit, and undo and strike.

**NixOS/Docker facts for the deployment fog:**
- `virtualisation.oci-containers.backend` defaults to **podman** when `stateVersion` is 22.05 or later. Set `backend = "docker"` and `virtualisation.docker.enable = true` explicitly.
- Each container becomes `docker-<name>.service` with `Restart=on-failure` and `autoStart = true`. `pull` defaults to `"missing"`. `imageFile`/`imageStream` can load a Nix-built image (`dockerTools.buildLayeredImage`/`streamLayeredImage`, which need no Docker daemon to build) instead of a Dockerfile image on `node:24-slim`.
- Configure the app with env vars: `PORT` (default 3000) and `HOST`. Set `PROTOCOL_HEADER`/`HOST_HEADER` only behind a trusted proxy. SvelteKit 3 config lives in `vite.config.js`, and the origin is `paths.origin` there.
- On SIGTERM the server waits up to `SHUTDOWN_TIMEOUT` (30 s), so the app must close its SSE streams on shutdown.
- The SQLite volume must be on **local disk**, because WAL doesn't work over network filesystems. **Backups must not copy the `.db` alone**, because the `-wal` file is part of the database state. Use the backup API or `VACUUM INTO`, or stop the container first.
- PWA install needs HTTPS (ticket 02).

Full findings and sources: [research/stack-on-home-server-v1-superseded.md](../research/stack-on-home-server-v1-superseded.md)


## Answer

**Recommendation: Go + htmx.** Build one static Go binary on the standard library (`net/http`, `html/template`; templ is optional and still v0.3.x). Store data in **SQLite through the pure-Go `modernc.org/sqlite`** in WAL mode on a host volume. Keep devices current with **Server-Sent Events**: after any log write (earn, redeem, undo, strike) the server sends a tiny `changed` event, and the htmx `sse` extension turns it into `hx-get` refetches of the pots and log. Pots never below zero, the once-per-kid-per-day daily-chore limit, instant redeem, and undo and strike are plain Go functions inside one SQLite transaction. The daily-chore limit also gets a unique index. Vendor htmx 2.0.x as a single file, so there's no npm and no JS build step. Nix builds the image (`buildGoModule` + `dockerTools.streamLayeredImage`), and `oci-containers` runs it with `backend = "docker"`.

**Runner-up: SvelteKit 3 (`adapter-node`) on Node 26**, with `node:sqlite` and SSE. Pick it instead if you're at home in TypeScript and not in Go, because the technical gap is small. **Node 26 is still "Current" today.** The Release WG schedule has Active LTS starting **2026-10-28** and EOL on 2029-04-30, and Node says production should use only Active or Maintenance LTS releases. Docker Hub's `lts` tag still points at 24. Confirm 26 has reached LTS before building.

| | Live update, no extra infra | SQLite | Docker/NixOS | Maintenance and churn | Code size* | iPad reconnect |
|---|---|---|---|---|---|---|
| **Go + htmx** | Hand-written SSE (~40 lines) + htmx `sse` | `modernc` v1.x, pure Go | Static binary, Nix-built image | Go 1 promise. htmx 2 stays `latest` until 2027 | S–M | Browser + htmx backoff, plus ~10 lines on `visibilitychange` |
| **SvelteKit 3** | Hand-written SSE + `refreshAll()` | `node:sqlite` RC | Node image or `buildNpmPackage` | Kit 3.0 shipped 2026-10-01. Node bump about every 2 years | S–M | Browser, plus hand-written `visibilitychange` |
| React Router 8 | Same as SvelteKit | Same | Same | Majors in 2021, 2024 and Jun 2026 | M | Same as SvelteKit |
| Next.js 16 | Same, plus RSC refresh | Same | Docs focus on cache and skew | A major each year, 2-year maintenance | M–L | Same as SvelteKit |
| Phoenix LiveView | **Built in** (WebSocket + PubSub) | `ecto_sqlite3` 0.25 (pre-1.0) | `phx.gen.release --docker` | Bug fixes on the latest Elixir minor only | **S** | **Built in** (`pageshow`, `visibilitychange`, heartbeat) |
| Rails 8.1 + Hotwire | **Built in** (Turbo refresh + Solid Cable on SQLite) | First-class in Rails 8 | Generated Dockerfile | **1 year bug fixes, 2 years security** per minor | **S** | **Built in** (`ConnectionMonitor`) |
| Django 6 + htmx | Needs ASGI, and Channels + **Redis** across processes | Docs: "development alternative" | Python + uvicorn | Best (LTS to 2028, then 3 years per release) | M | As htmx |
| PocketBase | Built-in SSE | Embedded | One binary | **v0.40**, "NOT recommended for production critical applications yet" | S + separate front end | `EventSource` |

*Code-size estimates are inference.

**Deciding reasons:**
1. With the PWA gone, the app is server-rendered tap screens plus a "changed, refetch" nudge. That plays to server-first stacks and removes SvelteKit's v1 advantage, its service worker.
2. For years of one-person maintenance, Go's compatibility promise, a near-empty dependency list, and a vendored htmx file churn far less than an npm tree on a fresh SvelteKit major and a Node runtime bumped about every 2 years. They also beat Rails' yearly upgrade cadence.
3. A static, cgo-free binary is the simplest thing to package on NixOS: one `nixos-rebuild switch`, with no Dockerfile and no registry.
4. LiveView and Rails + Hotwire need the least code for live sync and handle iPad sleep and wake in their client libraries. They lose only on language reach (Elixir) and support windows (Rails). Django needs Redis for cross-process push. PocketBase rules itself out in its own docs. Next.js adds machinery this app doesn't need.

**Where it hinges on familiarity:** if Go and TypeScript are about equal for you, or you know neither, take Go. With TypeScript or Svelte (the Node 26 choice hints at this), take SvelteKit. With React, take React Router 8, not Next.js. With Elixir, take LiveView, which is the best live-sync fit. With Ruby, take Rails 8 + Hotwire, which is the least code, and budget for yearly upgrades. With Python, Django works but fights you on live updates.

**iPad Safari:** `EventSource` reconnects after network errors but gives up for good on a non-200 response or a non-`text/event-stream` one. So never serve an error page on the stream URL, and send a `:` keep-alive about every 15 s. Whatever the stack, refetch on `visibilitychange`/`pageshow` and reopen the stream after the iPad wakes. Only Phoenix and Action Cable do this for you.

**NixOS/Docker facts for the deployment fog:**
- The `oci-containers` backend defaults to **podman** when `stateVersion` is 22.05 or later. Set `backend = "docker"` and `virtualisation.docker.enable = true`.
- The unit is `docker-pebblepot.service`, with `Restart=on-failure` and `TimeoutStopSec=120`. Stop runs `docker stop`, which sends SIGKILL after about 10 s, so close SSE streams promptly on SIGTERM. `autoStart = true`.
- For Go, `buildGoModule` (`vendorHash`) feeds `dockerTools.streamLayeredImage`, which feeds `imageStream` (loaded with `docker load` at service start). No Dockerfile and no registry. Include `tzdata` and set `TZ`, because "once per day" means the household's local day.
- For the Node runner-up, use a multi-stage Dockerfile on `node:26-slim` (`npm ci --omit dev`, `node build`), or `buildNpmPackage` (`npmDepsHash` changes with the lockfile).
- Volume: `/var/lib/pebblepot:/data` on **local disk**. WAL fails on network filesystems. Never copy the `.db` without its `-wal` file. Back up with `VACUUM INTO` on a systemd timer, or the online-backup API, or stop the container and copy the directory.
- Plain HTTP: bind the port to the LAN and WireGuard addresses only. Settle on the hostname(s) devices use. SvelteKit needs `paths.origin`/`csrf.trustedOrigins`, because it assumes `https` otherwise and form actions fail. Phoenix needs `check_origin` and an HTTP `url`. Rails needs `force_ssl`/`assume_ssl` off. Go's `CrossOriginProtection` needs nothing.

Full findings and sources: [research/stack-on-home-server.md](../research/stack-on-home-server.md)

## Comments

**2026-10-07, PWA dropped:** v1 is a plain responsive web app served over HTTP, with no service worker and no web manifest. Everything else in the recommendation stands. SSE works over plain HTTP, and the last bullet about HTTPS no longer applies.

**2026-10-07, Node 26 instead of 24:** the user chose Node 26 (`node:26-slim`) over Node 24. Node 26 is still "Current" today and should become LTS in late October 2026, before any building starts. Confirm it has reached LTS at build time. Nothing else changes: `node:sqlite` is built into both.

**2026-10-07, confirmed:** the user confirmed Go + htmx. The earlier note about Node 26 no longer applies.

**2026-10-09, Rust instead of Go:** the user chose Rust + htmx over the recommended Go + htmx. The rest of the architecture carries over: SQLite in WAL mode on local disk, SSE "changed" nudges plus a refetch on wake, domain rules in one transaction, and a Nix-built image under `oci-containers` with the Docker backend. Which Rust libraries to use is answered in [Which Rust libraries should Pebblepot use?](05-rust-crates.md).
