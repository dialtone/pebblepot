# Which stack should run Pebblepot on the NixOS home server? (v2)

Research for [issues/01-stack-on-home-server.md](../issues/01-stack-on-home-server.md). This replaces [stack-on-home-server-v1-superseded.md](stack-on-home-server-v1-superseded.md), which compared only SvelteKit and PocketBase and still assumed a PWA. Sources were read on 2026-10-07. Every factual claim cites a primary source from [Sources](#sources). Anything I inferred is marked *(inference)*.

## Recommendation in one paragraph

Use **Go + htmx**: one Go binary built on the standard library's `net/http` and `html/template` (templ is optional), **SQLite through the pure-Go `modernc.org/sqlite` driver** in WAL mode on a host volume, and **Server-Sent Events** to send a "changed" nudge. When the nudge arrives, htmx re-fetches the pots and log fragments. Ship it as a tiny image that Nix builds (`buildGoModule` + `dockerTools`), run by `virtualisation.oci-containers` with `backend = "docker"`. **Runner-up: SvelteKit 3 on Node 26** (Node 26 is due to become LTS on 2026-10-28). Choose it over Go if TypeScript is the language you're at home in and Go isn't. On the criteria in this ticket the gap between the two is small, and familiarity can outweigh it. See [Where the choice hinges on familiarity](#where-the-choice-hinges-on-familiarity).

## What changed since v1

- **No PWA.** This removes SvelteKit's main structural advantage in v1, its built-in service worker. Everything is now server-rendered pages plus a live nudge, which is where server-first stacks (Go + htmx, Rails + Hotwire, LiveView) are strongest. *(inference)*
- **Plain HTTP.** Browsers only speak HTTP/2 over TLS, so the HTTP/1.1 limit of 6 SSE connections per origin per browser applies. At one tab per device that is enough [S-MDN-SSE]. *(inference for the TLS-only point)* Several frameworks assume HTTPS by default and need a config change to work over HTTP (see the table and per-candidate notes).
- **Node 26 instead of 24** matters only if the stack is Node-based. See [Node 26 status](#node-26-status).

## Requirements recap

Pebblepot is a responsive web app, designed iPad-first (Safari, portrait and landscape) and usable on phones, served over plain HTTP on the LAN and WireGuard. A tap on one device (earn, redeem, undo, strike) must update the pots and log on the others promptly. Domain rules live on the server: pots never below zero, a daily chore once per kid per day, instant redeem, and undo and strike. The data is a few dozen log rows a day and must be stored reliably with a sane backup. The app runs as a Docker container on x86-64 NixOS, built and maintained for years by one developer. *(from the ticket and map)*

## Comparison

| Candidate | Live update without extra infra | SQLite | Docker / NixOS | Maintenance policy and churn | Code for this app *(inference)* | iPad Safari reconnect |
|---|---|---|---|---|---|---|
| **Go + htmx** (recommended) | SSE by hand in `net/http` (`ResponseController.Flush`) plus an in-process fan-out, about 40 lines. The htmx `sse` extension turns events into `hx-get` refetches [S-GO-RC][S-HTMX-SSE] | `modernc.org/sqlite` v1.x, pure Go (no cgo), bundles SQLite 3.53.4 [S-MODERNC] | Static binary. `buildGoModule` + `dockerTools` give a small image with no Dockerfile [S-NIX-GO][S-NIX-DT] | Go 1 compatibility promise. Each release is supported until two newer ones ship, about 1 year [S-GO-COMPAT][S-GO-REL]. htmx is one vendored JS file. 2.x is still `latest`, and 4.0 becomes `latest` in 2027 [S-HTMX] | Small to medium. Handlers, SQL and templates are written by hand, with almost no JS | `EventSource` reconnects on network errors [S-WHATWG-SSE], and the htmx extension adds its own exponential backoff [S-HTMX-SSE]. Add a few lines that refetch on `visibilitychange` *(inference)* |
| **SvelteKit 3** (runner-up) | SSE from a `+server.ts` stream plus an in-process emitter. The client calls `refreshAll()` on each event *(inference)* | `node:sqlite` is built in, Stability 1.2 "Release candidate" in Node 26 docs [S-NODE-SQLITE] | `adapter-node`: `node build`, `npm ci --omit dev` [S-SK-NODE]. Nix: `buildNpmPackage` (needs `npmDepsHash`) [S-NIX-NPM] | Kit 1 in Dec 2022, 2 in Dec 2023, **3 on 2026-10-01** [S-NPM-KIT]. Node LTS lasts about 30 months, so a runtime bump about every 2 years [S-NODE-REL] | Small to medium. One TypeScript codebase, but more client JS | Same `EventSource` behaviour. The `visibilitychange` refetch is hand-written *(inference)* |
| React Router 8 (framework mode) | Like SvelteKit: a resource route streams SSE, and the client calls `useRevalidator` [S-RR-MODES] | Same as SvelteKit (Node) | Same as SvelteKit (Node) | Majors in Nov 2021 (6), Nov 2024 (7) and **Jun 2026 (8)** [S-NPM-RR] | Medium (React plus loaders and actions) | Same as SvelteKit |
| Next.js 16 | Route handler SSE plus `router.refresh()` *(inference)* | Same as SvelteKit (Node) | `next start` in Docker. The docs cover cache, ISR, encryption keys and version skew [S-NEXT-SELF] | A major every October. Maintenance lasts 2 years after release [S-NEXT-SUP] | Medium to large (RSC and the caching model) | Same as SvelteKit |
| Phoenix LiveView | **Built in**: a WebSocket plus `Phoenix.PubSub` push re-rendered diffs [S-LV] | `mix phx.new --database sqlite3` uses `ecto_sqlite3` [S-PHX-NEW], which is at v0.25 (pre-1.0) [S-HEX] | `mix phx.gen.release --docker`, Debian base [S-PHX-REL]. Nix `mixRelease` [S-NIX-MIX] | Elixir minors about twice a year. Only the latest minor gets bug fixes, and the last 5 get security fixes [S-EX-COMPAT] | **Smallest**: live sync is close to free | **Best**: the client reconnects on `visibilitychange` and `pageshow`, with a 30 s heartbeat and backoff [S-PHX-JS], and LiveView reconnects with exponential backoff and recovers forms [S-LV-DEPLOY] |
| Rails 8.1 + Hotwire | **Built in**: Turbo `broadcasts_refreshes` (morph) over Action Cable with **Solid Cable on SQLite** (it polls the table, by default every 0.1 s), so no Redis [S-TURBO][S-SOLIDCABLE] | First-class in Rails 8: "suitable for real production use" [S-RAILS8] | Generated Dockerfile plus Thruster [S-RAILS8]. Nix packaging of gems is awkward, so build with the Dockerfile *(inference)* | Each minor gets **1 year of bug fixes and 2 years of security fixes**. 8.1 bug fixes end 2026-10-10 [S-RAILS-MAINT] | **Small**: generators and conventions | **Good**: `ConnectionMonitor` reopens on `visibilitychange` and after a 6 s stale threshold [S-AC-MONITOR] |
| Django 6 + htmx | Needs ASGI for SSE [S-DJ-STREAM]. Cross-process push needs Channels plus **Redis**, because the in-memory layer says "Do Not Use In Production" [S-CHANNELS] | Docs frame SQLite as "an excellent development alternative" and warn it gets "too lite" [S-DJ-DB] | Plain Python image plus uvicorn *(inference)* | Strong: 5.2 LTS runs to Apr 2028, and from 2028 every release gets 3 years [S-DJ-DL] | Medium | Same as the htmx SSE row, if built |
| PocketBase | **Built in** SSE realtime [S-PB-RT] | Embedded | One binary, simple [S-PB-PROD] | **v0.40.x**. "Full backward compatibility is not guaranteed before reaching v1.0.0"; "NOT recommended for production critical applications yet" [S-PB][S-GOPROXY-PB] | Small backend, but a separate front end plus goja JS hooks | `EventSource`-based |

## Per-candidate notes

### Go + htmx (recommended)

- **Runtime.** The Go 1 promise: programs "will continue to compile and run correctly, unchanged" [S-GO-COMPAT]. Each major release is supported "until there are two newer major releases" [S-GO-REL]. The latest is go1.27.1 (2026-09-01) [S-GO-REL]. Upgrading is usually a one-line `go.mod` change *(inference)*.
- **SSE in the standard library.** `http.NewResponseController(w).Flush()` flushes buffered data to the client [S-GO-RC]. `Server.Shutdown` "waits indefinitely for connections to return to idle" [S-GO-SERVER], so the SSE handlers must watch a shutdown context and return. Otherwise `docker stop` waits out its grace period *(inference)*.
- **CSRF without a framework.** Go's `http.CrossOriginProtection` rejects cross-origin non-safe requests using `Sec-Fetch-Site`, or falls back to comparing `Origin` with `Host` [S-GO-CSRF]. That comparison works over plain HTTP whatever hostname a device uses *(inference)*.
- **SQLite.** `modernc.org/sqlite` is "a CGo-free port of the C SQLite3 library", supports linux/amd64, and bundles SQLite 3.53.4. It runs about 1.3 to 2 times slower than C on CPU-bound work, which doesn't matter at a few dozen rows a day [S-MODERNC]. Set PRAGMAs (WAL, `busy_timeout`, `foreign_keys`) through the DSN or a connection hook, because pooled connections keep their state [S-MODERNC]. With no cgo, `CGO_ENABLED=0` gives a static binary *(inference)*.
- **htmx.** The `sse` extension uses `sse-connect`, and `hx-trigger="sse:changed"` plus `hx-get` refetches a fragment. It "adds its own reconnection logic (using an exponential-backoff algorithm)" [S-HTMX-SSE]. htmx 4.0 is out but deliberately not `latest` "so that people using the 2.x line are not accidentally upgraded", with the switch planned for 2027 [S-HTMX]. Vendor `htmx.min.js` 2.0.x and the extension into the repo. There's no npm and no build step *(inference)*.
- **Templates.** Use `html/template` (stdlib) to avoid any pre-1.0 dependency. templ is nicer (typed components) but is still **v0.3.x** [S-GOPROXY-TEMPL][S-TEMPL].
- **Domain rules** are plain Go functions in one SQLite transaction: check the pot, insert the log row, enforce the daily-chore uniqueness with a unique index on (kid, chore, local date). Undo and strike set a flag on the row *(inference)*.
- **Cost.** More hand-written plumbing than Rails or LiveView (routing, SQL, forms), but everything is explicit and few dependencies can churn *(inference)*.

### SvelteKit 3 (runner-up)

- **Fresh major.** `@sveltejs/kit` 3.0.0 shipped on 2026-10-01 alongside `adapter-node` 6.0.0. It's now 3.0.1 [S-NPM-KIT]. Config moved to `vite.config.js`, and "svelte.config.js … is no longer supported" [S-SK-NODE]. Expect tutorials and libraries to lag for a few months *(inference)*.
- **Plain-HTTP gotcha (sourced).** Unless `paths.origin` or `PROTOCOL_HEADER` is set, `adapter-node` derives the origin from `host` "and the `https` protocol" [S-SK-NODE]. The source falls back to `'https'` [S-SK-HANDLER]. Over plain HTTP, form actions would then fail with "Cross-site POST form submissions are forbidden" [S-SK-NODE]. Fix: set `paths.origin` to the one HTTP origin everyone uses, or list each LAN and WireGuard origin in `csrf.trustedOrigins` [S-SK-CONFIG].
- **SQLite.** `node:sqlite` is "Stability: 1.2 - Release candidate" (since v25.7.0) in the Node 26.11.1 docs. It has `sqlite.backup(sourceDb, path)`, an online backup that wraps `sqlite3_backup_*` [S-NODE-SQLITE].
- **Shutdown.** On SIGTERM it waits for in-flight requests for up to `SHUTDOWN_TIMEOUT` (30 s) [S-SK-NODE], so SSE streams must be closed on `sveltekit:shutdown` *(inference)*.
- **Why it's second:** it carries a fresh framework major, the npm dependency tree, a Node runtime bump about every 2 years, and an RC SQLite module. Go + htmx avoids all of these for an app whose UI is a handful of tap screens *(inference)*. It wins if a richer client UI (animations, optimistic updates) matters more than expected after the kid screens prototype, or if TypeScript is your language.

#### Node 26 status

- **Node 26 is "Current" today.** It was released 2026-05-05. The Release WG schedule has **Active LTS starting 2026-10-28**, maintenance from 2027-10-20, and **end of life on 2029-04-30**. "Dates are subject to change" [S-NODE-RELWG]. Node's guidance: "Production applications should only use Active LTS or Maintenance LTS releases" [S-NODE-REL].
- The Docker Hub `lts` tags still point at 24 (24.21.0). `26-slim` exists (26.10.0, trixie) [S-DOCKER-NODE]. Use `node:26-slim` once 26 is LTS. Confirm on the release page at build time.
- This applies to SvelteKit, React Router and Next.js only. The Go recommendation doesn't use Node.

### React Router 8 (framework mode): dropped in favour of SvelteKit

Its loaders, actions and `useRevalidator` [S-RR-MODES] cover the same ground as SvelteKit, on the same Node runtime, with the same SSE work. It had majors in 2021, 2024 and June 2026 [S-NPM-RR]. Choose it over SvelteKit only if you already know React *(inference)*.

### Next.js 16: dropped

Its support policy is fine: an Active LTS until the next major, then Maintenance LTS for 2 years from release [S-NEXT-SUP]. But the self-hosting guide is mostly about the cache, ISR, encryption keys, deployment IDs and version skew [S-NEXT-SELF], none of which a one-household app needs. It adds nothing for live updates over React Router *(inference)*.

### Phoenix LiveView: strongest technical fit, hinges on Elixir

- "LiveViews are processes that receive events, update their state, and render updates to a page as diffs" over a persistent connection [S-LV]. A `Phoenix.PubSub` broadcast after each log write updates every open device. This is the core requirement, with no hand-rolled SSE *(inference)*.
- **iPad behaviour is handled in the library.** `phoenix.js` disconnects on `pagehide`, reconnects on `pageshow` and `visibilitychange`, sends a heartbeat every 30 s, and backs off from 10 ms to 5 s [S-PHX-JS]. LiveView reconnects "using exponential back-off" and recovers forms [S-LV-DEPLOY].
- **SQLite** is a generator option (`--database sqlite3` → `ecto_sqlite3`) [S-PHX-NEW]. `ecto_sqlite3` is at **0.25.0** (pre-1.0) [S-HEX].
- **Plain HTTP:** the generated `runtime.exs` sets `url: [host: PHX_HOST, port: 443, scheme: "https"]` [S-PHX-RUNTIME]. Change it to HTTP, and allow every hostname devices use in `check_origin`, or the WebSocket is refused *(inference)*.
- **Support:** "Elixir applies bug fixes only to the latest minor branch". Security fixes cover the last five minors, which is about 2.5 years [S-EX-COMPAT]. The latest is Phoenix 1.8.15 / LiveView 1.2.12 [S-HEX].
- **Why not first:** the language is the least common on the shortlist. If you don't know Elixir/OTP, the learning cost outweighs the code it saves for an app this small *(inference)*.

### Rails 8 + Hotwire: strongest batteries-included option, hinges on Ruby

- Rails 8 made SQLite "suitable for real production use". Solid Cable "replaces the need for Redis", and the release ships a tuned Dockerfile plus Thruster so "there's no need to put an Nginx … in front" [S-RAILS8].
- Solid Cable "keeps messages in a table and continuously polls for updates" (`polling_interval` 0.1 s in the example) and is "tested to work with … SQLite" [S-SOLIDCABLE]. Turbo `broadcasts_refreshes` plus `turbo_stream_from` morph-refreshes every subscribed page [S-TURBO]. Live sync is about two lines *(inference)*.
- The Action Cable client reconnects on `visibilitychange` and when no ping arrives within 6 s [S-AC-MONITOR].
- **Plain HTTP:** the production template sets `assume_ssl` and `force_ssl = true` only when Kamal is skipped [S-RAILS-PROD]. Make sure both are off.
- **Why not first:** the maintenance window is short. Each minor gets 1 year of bug fixes and 2 years of security fixes, and 8.1's bug-fix window closes on 2026-10-10 [S-RAILS-MAINT]. That means a framework upgrade about once a year, plus the Ruby and gem churn *(inference)*. It's a good pick if you already know Rails.

### Django 6 + htmx: dropped

SSE needs ASGI, because under WSGI "streaming responses will tie a worker process for the entire duration" [S-DJ-STREAM]. Fan-out across processes needs Channels, and its only production layer is Redis: the in-memory layer says "Do Not Use In Production" [S-CHANNELS]. Django's own docs frame SQLite as a development alternative [S-DJ-DB]. A single ASGI worker with an in-process queue would work *(inference)*, but it fights the framework's defaults. Its support policy is the best on the list [S-DJ-DL], so if you're a Python developer this is a workable choice, but it is not the cheapest.

### PocketBase: dropped

Its docs rule it out for a long-lived app: "full backward compatibility is not guaranteed before reaching v1.0.0" and "NOT recommended for production critical applications yet" [S-PB]. It is still at v0.40.4 [S-GOPROXY-PB]. The JS hooks run on goja, "an embedded ES5 JavaScript engine … not fully spec compliant", with no `setTimeout` and no Node APIs [S-PB-JS]. Pebblepot's rules would split across hooks, API rules and a separate front end.

## Live updates on iPad Safari (all candidates)

- The spec: on a network error, `EventSource` will "reestablish the connection". If the status isn't 200 or the type isn't `text/event-stream` it "fail[s] the connection" and "does not attempt to reconnect". Authors may send a `:` comment line "every 15 seconds or so" to keep proxies from dropping the connection [S-WHATWG-SSE]. The server must never answer the SSE URL with an error page, for example during a restart behind a proxy. Otherwise the stream stays dead until the page reloads *(inference)*.
- iPadOS suspends pages when the screen locks or Safari is backgrounded. A suspended TCP connection may look open to the client after wake *(inference; no first-party WebKit source found)*. So, whatever the stack, on `visibilitychange` to visible (and on `pageshow`) the client should refetch the pots and log and, for SSE, reopen the `EventSource`. Phoenix and Action Cable do this in their client libraries [S-PHX-JS][S-AC-MONITOR]. With SSE stacks it is about 10 lines of hand-written JS *(inference)*.
- Use "something changed, refetch" events rather than diffs, so a missed event can never leave a device wrong after it reconnects *(inference)*.

## Where the choice hinges on familiarity

- **Go and TypeScript about equal, or neither:** take Go + htmx.
- **TypeScript or Svelte, not Go:** take SvelteKit. The Node 26 choice suggests you are comfortable with Node *(inference)*. The technical gap is small: most of it is deploy simplicity and maintenance churn.
- **React:** take React Router 8 instead of SvelteKit, not Next.js.
- **Elixir:** take Phoenix LiveView, which is the best fit for live sync.
- **Ruby:** take Rails 8 + Hotwire, which needs the least code. Budget for a yearly upgrade.
- **Python:** Django works, but expect friction with live updates.

## NixOS / Docker packaging facts (for the deployment fog)

- **The backend defaults to podman.** It is `"podman"` when `system.stateVersion` is 22.05 or later. Set `virtualisation.oci-containers.backend = "docker"` and `virtualisation.docker.enable = true` [S-NIX-OCI].
- **Unit:** `serviceName` defaults to `"${backend}-${name}"`, so `docker-pebblepot.service`. It has `Restart = "on-failure"`, `TimeoutStopSec = 120`, and `autoStart = true` adds it to `multi-user.target`. Stop runs `docker stop <name>` [S-NIX-OCI]. `docker stop` sends SIGTERM and then SIGKILL after its default grace period (10 s), so the app must close SSE streams quickly *(inference; Docker CLI default)*.
- **Images:** set `image` and `pull` (default `"missing"`), or load a Nix-built image with `imageFile` or `imageStream` (the loader runs `docker load`) [S-NIX-OCI]. `dockerTools.buildLayeredImage` / `streamLayeredImage` build images without Docker [S-NIX-DT].
  - **Go:** `buildGoModule` (pinned with `vendorHash`) [S-NIX-GO] → `streamLayeredImage { contents = [ app cacert tzdata ]; config.Cmd = [ "/bin/pebblepot" ]; }` → `imageStream`. Then `nixos-rebuild switch` builds and deploys, and there is no registry and no Dockerfile *(inference)*. Include `tzdata` and set `TZ`, because "once per kid per day" depends on the household's local date *(inference)*.
  - **Node (runner-up):** a Dockerfile on `node:26-slim` (multi-stage `npm ci`, `npm run build`, `npm ci --omit dev`, `CMD ["node","build"]`) [S-SK-NODE]. Or `buildNpmPackage`, which needs an `npmDepsHash` that changes with every lockfile change [S-NIX-NPM].
- **Volume:** `volumes = [ "/var/lib/pebblepot:/data" ]` on **local disk**. WAL "will not work on a network filesystem", and the `-wal` file "is part of the persistent state", so never copy the `.db` alone [S-SQLITE-WAL]. Back up with `VACUUM INTO` on a systemd timer, or the online-backup API (Node's `sqlite.backup`) [S-NODE-SQLITE]. Or stop the container and copy the directory *(inference)*.
- **Port:** bind to the LAN and WireGuard addresses only (`ports = [ "192.168.x.y:8080:8080" ]`). No TLS and no reverse proxy are needed *(inference from the map)*.
- **Host and origin:** decide on one hostname, or a list, that devices use on the LAN and over WireGuard. SvelteKit `paths.origin`/`csrf.trustedOrigins`, Phoenix `check_origin` and Rails `config.hosts` need it. Go's `CrossOriginProtection` doesn't *(inference)*.

## Sources

- [S-GO-REL] Go release history and policy: https://go.dev/doc/devel/release
- [S-GO-COMPAT] Go 1 compatibility promise: https://go.dev/doc/go1compat
- [S-GO-RC] Go source, `net/http/responsecontroller.go`: https://github.com/golang/go/blob/master/src/net/http/responsecontroller.go
- [S-GO-SERVER] Go source, `net/http/server.go` (`Server.Shutdown`): https://github.com/golang/go/blob/master/src/net/http/server.go
- [S-GO-CSRF] Go source, `net/http/csrf.go` (`CrossOriginProtection`): https://github.com/golang/go/blob/master/src/net/http/csrf.go
- [S-MODERNC] modernc.org/sqlite package docs: https://pkg.go.dev/modernc.org/sqlite
- [S-TEMPL] templ docs: https://templ.guide/
- [S-GOPROXY-TEMPL] Go module proxy, templ @latest (v0.3.1070, 2026-10-04): https://proxy.golang.org/github.com/a-h/templ/@latest
- [S-HTMX] htmx home page (versions, htmx 4 status): https://htmx.org/
- [S-HTMX-SSE] htmx SSE extension: https://htmx.org/extensions/sse/
- [S-SK-NODE] SvelteKit docs, adapter-node: https://svelte.dev/docs/kit/adapter-node
- [S-SK-CONFIG] SvelteKit docs, configuration (`csrf`, `paths.origin`): https://svelte.dev/docs/kit/configuration
- [S-SK-HANDLER] SvelteKit source, `packages/adapter-node/src/handler.js` (`get_origin`): https://github.com/sveltejs/kit/blob/main/packages/adapter-node/src/handler.js
- [S-NPM-KIT] npm registry, `@sveltejs/kit` and `@sveltejs/adapter-node` versions and times (`npm view … time`)
- [S-NODE-REL] Node.js releases page: https://nodejs.org/en/about/previous-releases
- [S-NODE-RELWG] Node.js Release WG schedule: https://github.com/nodejs/Release
- [S-NODE-SQLITE] Node.js v26 docs, `node:sqlite`: https://nodejs.org/api/sqlite.html
- [S-DOCKER-NODE] Docker Hub official `node` image tags: https://hub.docker.com/_/node
- [S-RR-MODES] React Router docs, picking a mode (v8.4.0): https://reactrouter.com/start/modes
- [S-NPM-RR] npm registry, `react-router` versions and times
- [S-NEXT-SUP] Next.js support policy: https://nextjs.org/support-policy
- [S-NEXT-SELF] Next.js docs, self-hosting (v16.4.0): https://nextjs.org/docs/app/guides/self-hosting
- [S-LV] Phoenix LiveView docs, welcome: https://hexdocs.pm/phoenix_live_view/welcome.html
- [S-LV-DEPLOY] Phoenix LiveView docs, deployments and recovery: https://hexdocs.pm/phoenix_live_view/deployments.html
- [S-PHX-JS] Phoenix source, `assets/js/phoenix/socket.js`: https://github.com/phoenixframework/phoenix/blob/main/assets/js/phoenix/socket.js
- [S-PHX-NEW] Phoenix docs, `mix phx.new`: https://hexdocs.pm/phoenix/Mix.Tasks.Phx.New.html
- [S-PHX-REL] Phoenix docs, deploying with releases: https://hexdocs.pm/phoenix/releases.html
- [S-PHX-RUNTIME] Phoenix installer template, `runtime.exs.eex`: https://github.com/phoenixframework/phoenix/blob/main/installer/templates/phx_single/config/runtime.exs.eex
- [S-EX-COMPAT] Elixir docs, compatibility and deprecations (v1.20.4): https://hexdocs.pm/elixir/compatibility-and-deprecations.html
- [S-HEX] hex.pm API, `phoenix` 1.8.15, `phoenix_live_view` 1.2.12, `ecto_sqlite3` 0.25.0: https://hex.pm/api/packages/
- [S-RAILS8] Rails blog, "Rails 8.0: No PaaS Required": https://rubyonrails.org/2024/11/7/rails-8-no-paas-required
- [S-RAILS-MAINT] Rails maintenance policy: https://rubyonrails.org/maintenance
- [S-RAILS-PROD] Rails source, production.rb template: https://github.com/rails/rails/blob/main/railties/lib/rails/generators/rails/app/templates/config/environments/production.rb.tt
- [S-SOLIDCABLE] Solid Cable README: https://github.com/rails/solid_cable
- [S-AC-MONITOR] Rails source, Action Cable `connection_monitor.js`: https://github.com/rails/rails/blob/main/actioncable/app/javascript/action_cable/connection_monitor.js
- [S-TURBO] Turbo handbook, page refreshes: https://turbo.hotwired.dev/handbook/page_refreshes
- [S-DJ-DL] Django download page (supported versions, new release policy): https://www.djangoproject.com/download/
- [S-DJ-STREAM] Django docs, request/response (`StreamingHttpResponse`): https://docs.djangoproject.com/en/stable/ref/request-response/
- [S-DJ-DB] Django docs, database notes (SQLite): https://docs.djangoproject.com/en/stable/ref/databases/
- [S-CHANNELS] Django Channels docs, channel layers: https://channels.readthedocs.io/en/latest/topics/channel_layers.html
- [S-PB] PocketBase docs, introduction: https://pocketbase.io/docs/
- [S-PB-JS] PocketBase docs, JS overview: https://pocketbase.io/docs/js-overview/
- [S-PB-RT] PocketBase docs, realtime API: https://pocketbase.io/docs/api-realtime/
- [S-PB-PROD] PocketBase docs, going to production: https://pocketbase.io/docs/going-to-production/
- [S-GOPROXY-PB] Go module proxy, pocketbase @latest (v0.40.4, 2026-09-12): https://proxy.golang.org/github.com/pocketbase/pocketbase/@latest
- [S-WHATWG-SSE] HTML Living Standard, server-sent events: https://html.spec.whatwg.org/multipage/server-sent-events.html
- [S-MDN-SSE] MDN, using server-sent events: https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events/Using_server-sent_events
- [S-SQLITE-WAL] SQLite, write-ahead logging: https://www.sqlite.org/wal.html
- [S-NIX-OCI] nixpkgs source, `nixos/modules/virtualisation/oci-containers.nix`: https://github.com/NixOS/nixpkgs/blob/nixos-unstable/nixos/modules/virtualisation/oci-containers.nix
- [S-NIX-DT] Nixpkgs manual source, dockerTools: https://github.com/NixOS/nixpkgs/blob/master/doc/build-helpers/images/dockertools.section.md
- [S-NIX-GO] nixpkgs source, `pkgs/build-support/go/module.nix` (`buildGoModule`): https://github.com/NixOS/nixpkgs/blob/nixos-unstable/pkgs/build-support/go/module.nix
- [S-NIX-NPM] nixpkgs source, `pkgs/build-support/node/build-npm-package/default.nix`: https://github.com/NixOS/nixpkgs/blob/nixos-unstable/pkgs/build-support/node/build-npm-package/default.nix
- [S-NIX-MIX] nixpkgs source, `pkgs/development/beam-modules/mix-release.nix`: https://github.com/NixOS/nixpkgs/blob/nixos-unstable/pkgs/development/beam-modules/mix-release.nix
