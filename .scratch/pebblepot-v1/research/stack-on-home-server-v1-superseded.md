# Which stack should run Pebblepot on the NixOS home server?

Research for [issues/01-stack-on-home-server.md](../issues/01-stack-on-home-server.md). Sources were read on 2026-10-07. Every claim is tied to a primary source in [Sources](#sources). Anything I inferred rather than read is marked *(inference)*.

## Recommendation in one paragraph

Use **SvelteKit 3 with `@sveltejs/adapter-node`, on Node.js 24 LTS**. Store data in **SQLite through Node's built-in `node:sqlite`**, in WAL mode, with the file on a host volume. Keep devices current with **Server-Sent Events** (SSE). Use **SvelteKit's built-in service worker** and a static web manifest for the PWA. Ship it as one Docker image, run by NixOS `virtualisation.oci-containers` with `backend = "docker"`.

The runner-up is **PocketBase** (a single Go binary with SQLite and built-in SSE realtime) behind a static PWA front end.

## Requirements recap (from the ticket and map)

- An installable PWA for phones and a shared tablet, reached over the LAN and WireGuard only.
- When one device taps (earn, redeem, undo, strike), every other device's pots and log update.
- A tiny single-household log that must be stored reliably.
- One developer, so build and maintenance must be cheap.

## Server runtime and framework: SvelteKit + adapter-node

- **Runs as a plain Node server.** `npm run build` writes the server to `build/`, and `node build` starts it. Packages listed in `dependencies` stay external, so the image needs `npm ci --omit dev`. Dev dependencies are bundled at build time. [S1]
- **Configured with env vars, which suits containers.** `PORT` defaults to 3000 and `HOST` defaults to `0.0.0.0`. `PROTOCOL_HEADER` and `HOST_HEADER` (and `ADDRESS_HEADER`/`XFF_DEPTH`) should be set only behind a trusted reverse proxy. `BODY_SIZE_LIMIT` defaults to 512kb. `.env` files are not loaded in production, so use the container's environment or `node --env-file`. [S1]
- **Shuts down gracefully.** On `SIGTERM` or `SIGINT` the server stops accepting requests and lets in-flight ones finish. It force-closes whatever is left after `SHUTDOWN_TIMEOUT` (default 30 s), then emits `sveltekit:shutdown`. [S1] SSE streams never finish on their own. The app should close them on shutdown, or every `docker stop` will wait the full timeout. *(inference)*
- **SvelteKit 3 changed its config.** Config now lives in `vite.config.js`, passed as `sveltekit({ adapter: adapter() })`. The docs say `svelte.config.js` "is no longer supported". The app's origin for CSRF checks is now `paths.origin` in that config. If it is unset, the origin is derived from the request. [S1] Older tutorials that use `svelte.config.js` or an `ORIGIN` env var are out of date.
- **One codebase covers everything.** Kid screens, parent screens, API endpoints (`+server.ts`) and the SSE endpoint all live in one TypeScript project. This is the main cost saving for one developer. *(inference)*

### Node version

Node 24 (Krypton) is the Active LTS, and Node 26 is "Current". Node.js says production should use only Active or Maintenance LTS releases. [S2] Base the image on Node 24 now. Node 26 should become LTS in late October 2026 under the usual even-release schedule *(inference)*, and moving to it then is a one-line change.

## Database: SQLite via `node:sqlite`

- **Built into Node, so there is no native addon.** `node:sqlite` was added in v22.5.0 and unflagged in v22.13.0/v23.4.0. It became **Stability 1.2, Release candidate** in v24.15.0. It needs no flag and is imported as `node:sqlite` (`DatabaseSync`, `exec`, `prepare`). [S3] [S4] This avoids `better-sqlite3`'s native build, so the image needs no compiler toolchain. *(inference)* The trade-off is that the module is RC rather than Stable. It is used only through `prepare`/`run`/`exec`, which are small and stable in shape. If it ever regresses, `better-sqlite3` is the fallback.
- **WAL mode suits the access pattern.** "Readers do not block writers and a writer does not block readers", and WAL needs fewer `fsync()` calls. [S5]
- **WAL needs a local disk.** It "will not work on a network filesystem", because all processes must share memory on the same host. [S5] Put the volume on the server's local disk, not NFS or SMB.
- **Backups must include the WAL.** "The WAL file is part of the persistent state of the database." Copying `pebblepot.db` without its `-wal` file can lose committed transactions or corrupt the database. [S5] The backup design (the map's "Deployment and data safety" fog) should use SQLite's online backup or `VACUUM INTO` to a snapshot file, or stop the container before copying the directory. A plain `cp` of the `.db` file is not safe.
- **The data is tiny, so it fits.** One household writes a few dozen log rows a day, all from one server process. The pot balance is the sum of its log, which is one `SUM()` query, and the daily-chore once-per-kid-per-day rule is one indexed lookup. A client-server database such as Postgres would add a second container and backup story for no benefit. *(inference)*

## Live updates: SSE, not polling or WebSockets

- **Updates only flow one way.** Devices send taps as ordinary POST requests, and only the server-to-device "something changed" notice needs push. SSE is built for exactly that. `EventSource` reconnects automatically when the connection drops, the server can set the `retry` delay, and the `id` field sets the last event ID. A comment line (`:`) works as a keep-alive. [S6] WebSockets would add a bidirectional protocol that Pebblepot doesn't need. *(inference)*
- **The connection limit doesn't bite here.** Over HTTP/1.x a browser allows only 6 SSE connections per domain across all tabs. HTTP/2 negotiates up to about 100 streams. [S6] Each phone or tablet has one Pebblepot tab, so 6 is enough. If the HTTPS reverse proxy from ticket 02 speaks HTTP/2, the limit goes away. *(inference)*
- **Suggested shape:** after any log write, the server sends a tiny `changed` event to every open stream. Clients then refetch the pots and recent log. They also refetch on `visibilitychange` and after a reconnect, which covers a phone waking from sleep when the stream may have dropped. This keeps one source of truth (the database) and needs no diffing. *(inference)*
- **Why not polling:** polling every few seconds also works at this scale and is the simplest option. But it makes taps feel laggy on other devices, or it wastes battery on the tablet. SSE costs about 30 lines more and gives instant updates. *(inference)*

## PWA

- SvelteKit bundles and **automatically registers** `src/service-worker.{js,ts}` (or `src/service-worker/index.ts`). `$app/manifest` exposes the build files, assets and prerendered pages to cache, and `version` comes from `$app/env` for naming caches. The service worker runs in production builds only. Browsers check for a new service worker on full navigations, and a new one does not take over until the old tabs close. [S7] The shared tablet may keep a stale build for a long time, so the app should call `registration.update()`, for example from the root layout. [S7]
- The Vite PWA plugin (Workbox) is the documented alternative if more advanced caching is needed. [S7]
- **Chrome installability** needs HTTPS and a manifest with `name`/`short_name`, 192px and 512px icons, `start_url`, and `display` set to `standalone` (or similar). It also needs some user engagement before the install prompt appears. [S8] How to get HTTPS on the LAN and WireGuard is ticket 02's question.

## Runner-up: PocketBase + static PWA

- **What it is:** an open-source backend with embedded SQLite, realtime subscriptions, auth, a superuser admin dashboard and a REST-style API, shipped as one executable. It can be extended with Go or JavaScript hooks. [S9]
- **Realtime is SSE.** `GET /api/realtime` opens the stream, `POST /api/realtime` sets subscriptions to collections or records, and the server disconnects idle clients after 5 minutes. [S10]
- **Its Docker story is simple.** The documented Dockerfile is Alpine-based and runs `pocketbase serve --http=0.0.0.0:8080`, with a volume at `/pb/pb_data`. It has built-in backups (Dashboard > Settings > Backups). Manual backups require stopping the app "for transactional safety". [S11]
- **Why it is second:** PocketBase's own docs warn that "full backward compatibility is not guaranteed before reaching v1.0.0" and advise against it for production-critical apps. [S9] Pebblepot's rules would also have to live in PocketBase hooks and API rules, beside a separate front-end project: pots never below zero, the daily-chore limit, undo and strike, and redeem taking effect immediately. With no sign-ins, the API rules would all be open anyway. That splits the logic across two places for little gain. *(inference)* It would win if the household wanted the admin dashboard as the parent screens, but ticket 04 plans in-app parent screens.

Options considered but not shortlisted *(inference)*: Next.js (heavier, with an ecosystem tuned for Vercel rather than self-hosting), and Go + htmx (excellent to deploy, but two languages and hand-rolled PWA and offline plumbing for one developer).

## NixOS / Docker packaging facts (for the deployment fog)

- **The default backend is podman, not Docker.** `virtualisation.oci-containers.backend` defaults to `"podman"` when `system.stateVersion` is 22.05 or later. Set `backend = "docker"` explicitly, along with `virtualisation.docker.enable = true`, since Docker is the stated preference. [S12]
- **Each container becomes a systemd unit.** It is named after `serviceName`, which defaults to `"${backend}-${name}"`, so `docker-pebblepot.service`. The unit has `Restart = "on-failure"` (no restart after a clean exit). `autoStart = true` adds it to `multi-user.target`. [S12]
- **Useful per-container options:** `image` (required), `imageFile`/`imageStream` (load a Nix-built image instead of pulling, and not both), `pull` (default `"missing"`; one of always/missing/never/newer), `volumes`, `ports`, `environment`, `environmentFiles`, `extraOptions`, `labels`, `networks`, and `login.*` for a private registry. [S12]
- **There are two ways to build the image:**
  1. **A Dockerfile** on `node:24-slim`: `npm ci`, `npm run build`, then `npm ci --omit dev` in the runtime stage, and `CMD ["node","build"]`. Build it on the server or push it to a registry, then point `image` at it. *(inference, from [S1])*
  2. **Nix `dockerTools.buildLayeredImage` / `streamLayeredImage`.** These build a Docker-compatible image (a `.tar.gz` in the store, or a script that streams one) with "Docker itself … not used" at build time. The result is loaded with `docker image load`. [S13] It pairs with `imageFile`/`imageStream` above, which makes the whole deployment one `nixos-rebuild switch`. *(inference)* This needs the npm build packaged as a Nix derivation, which is more upfront work.
- **Example volume:** `volumes = [ "/var/lib/pebblepot:/data" ]` on local disk (WAL needs it [S5]). Point the app at `/data/pebblepot.db`.
- **Example environment:** `PORT=3000` and `HOST=0.0.0.0`. Add `PROTOCOL_HEADER=x-forwarded-proto` and `HOST_HEADER=x-forwarded-host` only if it sits behind the reverse proxy from ticket 02. [S1]
- **No public exposure:** bind the port to the LAN or WireGuard interface, or to `127.0.0.1` behind the proxy. *(inference, from the map's environment note)*

## Open questions passed on

- HTTPS and certificates for install (ticket 02). Whether the proxy speaks HTTP/2 there affects the SSE limit above, though not in a way that matters.
- The backup method (`VACUUM INTO` on a timer versus stop-and-copy) and restore drill belong to the deployment and data safety fog.

## Sources

- [S1] SvelteKit docs, adapter-node: https://svelte.dev/docs/kit/adapter-node
- [S2] Node.js release table: https://nodejs.org/en/about/previous-releases
- [S3] Node.js docs, SQLite (latest): https://nodejs.org/api/sqlite.html
- [S4] Node.js v24 docs, SQLite: https://nodejs.org/docs/latest-v24.x/api/sqlite.html
- [S5] SQLite, Write-Ahead Logging: https://www.sqlite.org/wal.html
- [S6] MDN, Using server-sent events: https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events
- [S7] SvelteKit docs, Service workers: https://svelte.dev/docs/kit/service-workers
- [S8] web.dev, Installation criteria: https://web.dev/articles/install-criteria
- [S9] PocketBase docs, Introduction: https://pocketbase.io/docs/
- [S10] PocketBase docs, Realtime API: https://pocketbase.io/docs/api-realtime/
- [S11] PocketBase docs, Going to production: https://pocketbase.io/docs/going-to-production/
- [S12] nixpkgs source, `nixos/modules/virtualisation/oci-containers.nix`: https://github.com/NixOS/nixpkgs/blob/nixos-unstable/nixos/modules/virtualisation/oci-containers.nix
- [S13] Nixpkgs manual source, dockerTools: https://github.com/NixOS/nixpkgs/blob/master/doc/build-helpers/images/dockertools.section.md
