# Which Rust libraries should Pebblepot use?

Research for [issues/05-rust-crates.md](../issues/05-rust-crates.md). The architecture comes from [issues/01-stack-on-home-server.md](../issues/01-stack-on-home-server.md) and its research file [stack-on-home-server.md](stack-on-home-server.md): plain HTTP, iPad-first, SQLite in WAL mode on local disk, SSE "changed" nudges that trigger htmx refetches, domain rules in one transaction, and a Nix-built Docker image under `oci-containers` with the Docker backend. Sources were read on 2026-10-09. Versions and dates come from the crates.io API [S-CRATES] unless another source is cited. Anything I inferred is marked *(inference)*.

## Recommendation in one paragraph

Use **axum 0.8 on tokio 1.x**, with **tower-http 0.7** for its new `CsrfLayer` (a port of Go's `CrossOriginProtection`) and, if wanted, `ServeDir`. Use **askama 0.16** for compile-time-checked Jinja-style templates: its `block`/`blocks` attributes render one block of a page as an htmx partial. Use **rusqlite 0.40 with `bundled` and `backup`**, migrations by **rusqlite_migration 2.6**, and **tokio-rusqlite 0.8** as the async handle, so one dedicated thread owns the connection and every domain rule runs inside one `IMMEDIATE` transaction. Fan out with a **`tokio::sync::watch` version counter** (no lag, coalesces bursts) and stop every SSE stream through a **`tokio_util::sync::CancellationToken`** that also drives `with_graceful_shutdown`. Package with **crane** (`buildDepsOnly` + `buildPackage`, with a fileset that keeps `templates/`, `migrations/` and `static/`) into **`dockerTools.streamLayeredImage`**, linked dynamically against glibc. No musl and no static linking. Use **jiff 0.2** with `tzdb-bundle-always` for the household's local day. **Runner-ups:** actix-web 4 (SSE via `actix-web-lab`), sqlx 0.9 (async-native, `query!` macros, but a compile-time database or `.sqlx` cache), minijinja 3 (runtime templates with hot reload), and `rustPlatform.buildRustPackage` with `cargoLock.lockFile` instead of crane.

## Crate status as of 2026-10-09

| Crate | Latest | Released | Recent cadence | Notes |
|---|---|---|---|---|
| **axum** | 0.8.9 | 2026-04-14 | 0.6 Nov 2022, 0.7 Nov 2023, 0.8 Jan 2025 | `main` has an unreleased breaking changelog, so 0.9 is coming [S-AXUM-CL] |
| actix-web | 4.15.0 | 2026-08-21 | 4.x since 2022, minors every few months | SSE lives in `actix-web-lab` 0.29.4, which is "experimental" [S-AWL] |
| **tokio** | 1.53.2 | 2026-10-03 | LTS minors: **1.51.x until Mar 2027, 1.53.x until Sep 2027** [S-TOKIO] | |
| tokio-util | 0.7.20 | 2026-10-09 | | `CancellationToken` |
| tokio-stream | 0.1.19 | 2026-07-22 | | `WatchStream`, `BroadcastStream` (feature `sync`) |
| **tower-http** | 0.7.1 | 2026-08-31 | 0.6 Sep 2024, **0.7 Jun 2026** | 0.7 added `csrf` [S-THTTP-CL] |
| **askama** | 0.16.1 | 2026-09-04 | 0.13 Mar 2025, 0.14 Apr 2025, 0.15 Dec 2025, 0.16 Apr 2026; MSRV 1.88 | Rinja merged back in. `rinja` is `0.4.0+deprecated` [S-ASKAMA] |
| askama_web | 0.16.0 | 2026-04-29 | tracks askama | Features `axum-0.8`, `actix-web-4` |
| maud | 0.27.0 | 2025-02-02 | About one release a year, nothing in 20 months | |
| minijinja | 3.0.0 | **2026-10-08** | 1.0 Jun 2023, 2.0 Apr 2024, 3.0 yesterday | Runtime templates |
| **rusqlite** | 0.40.2 | 2026-08-08 | Breaking 0.x every 2 to 5 months (0.34 to 0.40 since Mar 2025) | Bundles SQLite **3.53.2** [S-RQ-H] |
| libsqlite3-sys | 0.38.2 | 2026-08-08 | Released with rusqlite | `links = "sqlite3"` |
| **rusqlite_migration** | 2.6.0 | 2026-05-28 | Several releases a year | Needs `rusqlite ^0.40` |
| **tokio-rusqlite** | 0.8.0 | 2026-09-06 | 0.6 Sep 2024, 0.7 Nov 2025, 0.8 Sep 2026 | Needs `rusqlite ^0.40.1` |
| deadpool-sqlite | 0.14.0 | 2026-08-26 | | Needs `rusqlite ^0.40` (alternative async handle) |
| sqlx | 0.9.0 | 2026-05-21 | 0.7 Jul 2023, 0.8 Jul 2024, 0.9 May 2026; **MSRV 1.94** | Repo moving to `transact-rs` [S-SQLX-CL] |
| diesel | 2.3.14 | 2026-10-07 | Monthly patches | Synchronous. Not pursued (see below) |
| jiff | 0.2.38 | 2026-10-06 | Frequent | |
| htmx.org (npm) | **2.0.11** (`latest`) | 2026-09-22 | 4.0.0 is on the `next` tag | [S-NPM-HTMX] |
| htmx-ext-sse (npm) | 2.2.4 | 2025-10-18 | | [S-HTMX-SSE] |
| crane | v0.24.0 | 2026-08-21 | Monthly-ish. Last commit 2026-10-02 | [S-CRANE-GH] |
| naersk | (no tagged releases) | | Last commit 2026-06-23 | |

The rustc that nixpkgs provides: **nixos-26.05's default `rustc` is 1.95.0**, with 1.97 and 1.98 also packaged, and nixos-unstable is on 1.98.1 [S-NIX-ALLPKGS]. That covers every MSRV above, including sqlx 0.9's 1.94.

## Web framework: axum 0.8 (runner-up actix-web 4)

**SSE.** `axum::response::sse::{Sse, Event, KeepAlive}` is built in. `KeepAlive::default()` sends a `:` comment every **15 s** (`max_interval: Duration::from_secs(15)`), and the timer resets whenever a real event goes out [S-AXUM-SSE]. That matches the WHATWG "every 15 seconds or so" advice [S-WHATWG-SSE]. The response sets `content-type: text/event-stream` and `cache-control` itself [S-AXUM-SSE].

**The empty-data trap.** The SSE spec says that when an event is dispatched, "If the data buffer is an empty string, set the data buffer and the event type buffer to the empty string and return" [S-WHATWG-SSE]. So an `event: changed` with no `data:` line never fires. Always send something, for example `Event::default().event("changed").data(version.to_string())` *(inference from the spec)*.

**Graceful shutdown and `docker stop`.** In 0.8.9, `axum::serve(...).with_graceful_shutdown(signal)` stops accepting, calls hyper's `graceful_shutdown()` on every connection, then **waits for all connection tasks to finish with no timeout** ("waiting for {} task(s) to finish"; `close_tx.closed().await`) [S-AXUM-SERVE-089]. An open SSE response is an in-flight request, so without help the server waits until Docker's SIGKILL about 10 s later *(inference)*. The fix is one `CancellationToken`. Cancel it on SIGTERM or Ctrl-C, pass `token.clone().cancelled_owned()` to `with_graceful_shutdown`, and end every SSE stream with `.take_until(token.cancelled_owned())` (`futures-util` `StreamExt`). Then the streams end, the connections close, and the process exits in milliseconds *(inference)*. Ending the stream is a clean EOF for the iPad. `EventSource` treats it as a network error and reconnects [S-WHATWG-SSE], and htmx's sse extension also retries with exponential backoff [S-HTMX-SSE].
- Unreleased in axum `main`: `ConnectionLifetimeLimits`/`MaxConnectionAge` with a `grace` cap. The changelog lists it under "Unreleased", next to several **breaking** changes, which means a 0.9 [S-AXUM-CL]. Don't rely on it. Expect one migration to 0.9 at some point *(inference)*.

**Static files.** `tower_http::services::ServeDir`/`ServeFile` (feature `fs`) serve a directory with range support. 0.7.1 added options such as `redirect_to_trailing_slash()` [S-THTTP-CL]. With only three or so assets (vendored `htmx.min.js` 2.0.11, `sse.js` 2.2.4 and one CSS file), embedding them with `include_bytes!` behind a small handler that sets `Content-Type` and a long `Cache-Control` on versioned filenames is just as simple. It also keeps the image down to one binary with no runtime path *(inference)*. Either way is fine. Pick `ServeDir` if the set grows.

**CSRF over plain HTTP.** axum has no CSRF of its own. **tower-http 0.7 added `CsrfLayer`**, "porting the cross-origin protection scheme introduced in Go 1.25" [S-THTTP-CL]. It allows a request if any of these hold [S-THTTP-CSRF]:
1. the method is GET, HEAD or OPTIONS;
2. `Origin` matches a trusted origin from an allow-list;
3. `Sec-Fetch-Site` is `same-origin` or `none`;
4. neither `Sec-Fetch-Site` nor `Origin` is present;
5. the authority in `Origin` matches the request's effective host byte for byte. This check is "byte-exact and scheme-blind" [S-THTTP-CSRF-SVC].

Rejections get a 403. **Plain-HTTP consequence:** Fetch Metadata headers are only added when the URL is potentially trustworthy ("If r's current URL is not an potentially trustworthy URL, return") [S-FETCH-META]. So on `http://pebblepot.lan` or `http://10.x.y.z`, Safari sends no `Sec-Fetch-Site`, and the layer falls back to the scheme-blind `Origin`/`Host` check in rule 5. That works with any hostname or IP a device uses, with no configuration, as long as no proxy rewrites `Host` (the module's own "Deployment caveat") [S-THTTP-CSRF] *(inference for the combination)*. htmx POSTs are same-origin `fetch`/XHR requests, so browsers attach `Origin` *(inference)*. No tokens are needed, and htmx needs no `hx-headers` setup. Pebblepot has no sign-ins, so CSRF only stops other sites from posting taps. That's cheap insurance *(inference)*.

**Compatibility.** axum 0.8.9 depends on `tower ^0.5.2` and `http ^1`. tower-http 0.7.1 depends on `tower ^0.5` and `http ^1` [S-CRATES], so its layers work on an axum `Router`. axum's own optional `tower-http ^0.6.8` dependency isn't enabled by axum's defaults (`form, http1, json, matched-path, original-uri, query, tokio, tower-log, tracing`) [S-CRATES] *(inference that no duplicate gets pulled in)*.

**Runner-up: actix-web 4.15.** It's mature and stable on 4.x. Its graceful shutdown has a built-in force-drop: "workers have this much time to finish serving requests. Workers still alive after the timeout are force dropped. By default shutdown timeout sets to 30 seconds" [S-ACTIX-SERVER]. Set `shutdown_timeout(5)` and SSE can't hold up `docker stop` even without a token. But its SSE responder lives in `actix-web-lab`, described as "Experimental extractors, middleware, and other extras for possible inclusion in Actix Web" [S-AWL], and tower-http's `CsrfLayer` doesn't plug into actix's middleware model *(inference)*. axum wins on ecosystem gravity (tokio-rs, about 11× actix-web's 90-day downloads [S-CRATES]), first-class SSE, and the tower-http CSRF layer.

## HTML templating: askama 0.16 (runner-up minijinja 3)

- **Compile-time checked.** Askama "generates type-safe Rust code from your templates at compile time based on a user-defined `struct`" [S-ASKAMA]. A typo in a field or a missing variable is a build error. That suits one developer maintaining the app for years *(inference)*.
- **htmx partials without separate files.** `#[template(path = "home.html", block = "pots")]` "renders the block by itself. Expressions outside of the block are not required by the struct… useful when you need to decompose your template for partial rendering". `blocks = ["pots", "log"]` generates `as_pots()`/`as_log()` sub-templates from one struct [S-ASKAMA-BOOK]. The full page and the `hx-get` refetch fragments come from the same markup, so they can't drift apart *(inference)*.
- **axum glue.** `askama_web` 0.16 with feature `axum-0.8` gives `#[derive(WebTemplate)]` → `IntoResponse`. The book suggests a custom error type instead if you want styled error pages [S-ASKAMA-FW]. Or call `.render()` and wrap it in `axum::response::Html` *(inference)*.
- **Health.** Rinja was a fork that "got merged back into Askama" [S-ASKAMA]. The rinja crate now ships `0.4.0+deprecated`. askama has had four breaking minors since March 2025 (0.13 → 0.16) [S-CRATES], so expect a small upgrade about twice a year *(inference)*.
- **Cost.** Every template edit recompiles the crate. For a handful of screens that's acceptable *(inference)*.
- **maud 0.27** (Rust-macro HTML) is also compile-time checked, but it has had no release in 20 months and averages about one a year [S-CRATES]. htmx attributes such as `hx-get` work in maud, but there's no block-reuse equivalent. You split functions by hand *(inference)*.
- **minijinja 3.0.0** came out 2026-10-08, one day before this research [S-CRATES]. It checks templates at runtime, so errors appear when a page renders. In exchange you get hot reload and no recompiles. It's a sound runner-up, but wait for a 3.0.x patch or two *(inference)*.

## SQLite access: rusqlite 0.40 + rusqlite_migration + tokio-rusqlite (runner-up sqlx 0.9)

**rusqlite (recommended).**
- Features: `bundled` ("Use bundled SQLite sources (instead of the one provided by your OS / distribution)") and `backup` ("hot-backup interface") [S-RQ-TOML]. The bundled amalgamation is SQLite **3.53.2** [S-RQ-H].
- **Backups:** `Connection::backup(name, dst_path, progress)` and the `backup::Backup` stepper wrap SQLite's online backup API. The module docs also point to `VACUUM INTO` [S-RQ-BACKUP]. Both satisfy the "never copy the `.db` without its `-wal`" rule [S-SQLITE-WAL]. Run either from a tiny admin endpoint or a `--backup <path>` subcommand of the same binary, called from a systemd timer through `docker exec` *(inference)*.
- **Transactions for the pot rules:** `TransactionBehavior::{Deferred, Immediate, Exclusive}`. "IMMEDIATE cause[s] the database connection to start a new write immediately" [S-RQ-TX]. Use `transaction_with_behavior(Immediate)` for earn, redeem, undo and strike. Check the pot, insert the log row, and let a unique index on (kid, chore, local date) enforce the daily limit, all in one transaction *(inference)*.
- **WAL and PRAGMAs:** set them once when the connection opens (`journal_mode=WAL`, `synchronous=NORMAL`, `foreign_keys=ON`, `busy_timeout`). The database must stay on local disk [S-SQLITE-WAL].
- **Migrations:** `rusqlite_migration` 2.6 tracks state in SQLite's `user_version` integer instead of a table ("much lighter as it is just an integer at a fixed offset"). It has an async example and an optional `include_dir` loader [S-RQM]. Embed migrations with `include_str!` and run them at startup *(inference)*.
- **Async:** rusqlite is synchronous. `tokio-rusqlite` 0.8 gives `Connection::call(|conn| …)`, which runs the closure on the connection's own background thread [S-TRQ]. One household writes a few dozen rows a day, so **one connection for everything** removes `SQLITE_BUSY` between pooled writers and keeps the rules strictly serialised *(inference)*. `deadpool-sqlite` 0.14 (`interact`) is the drop-in if you ever want a read pool [S-CRATES]. Bare `tokio::task::spawn_blocking` around an `Arc<Mutex<Connection>>` also works with zero extra crates *(inference)*.

**sqlx 0.9 (runner-up).**
- Async-native pool, `sqlx::migrate!`, and `query!` macros that check SQL against a database at compile time.
- `SqliteConnectOptions` turns on `foreign_keys` by default but deliberately leaves `journal_mode` alone ("Don't set `journal_mode` unless the user requested it"), so call `.journal_mode(SqliteJournalMode::Wal)` explicitly [S-SQLX-OPT].
- The `sqlite` feature bundles SQLite through `libsqlite3-sys` [S-SQLX-LIB].
- **Costs for this app:** the macros need a live `DATABASE_URL` or a `.sqlx` cache from `cargo sqlx prepare` "before every commit that adds or changes a query" [S-SQLX-FAQ]. The crane sqlx example does its build-time `sqlx database create` / `migrate run` [S-CRANE-SQLX]. sqlx also has no wrapper for the backup API (use `VACUUM INTO`) *(inference)*, MSRV is 1.94, and major versions take 1 to 2 years.
- **Can't mix with rusqlite 0.40:** sqlx-sqlite 0.9.0 requires `libsqlite3-sys >=0.30.1, <0.38.0` [S-SQLX-TOML], rusqlite 0.40 uses `libsqlite3-sys 0.38.2`, and "Only one version of `libsqlite3-sys` may appear in the dependency tree" because it `links = "sqlite3"` [S-SQLX-LIB][S-RQ-SYS]. Pick one SQLite crate.
- sqlx is the right pick if compile-checked SQL matters more to you than a simpler build *(inference)*.

**diesel 2.3** is actively patched, but it's synchronous and its typed-schema DSL is a lot of machinery for four tables *(inference)*. Not pursued.

## Live-update fan-out: `tokio::sync::watch` (runner-up `tokio::sync::broadcast`)

- **broadcast:** "If a value is sent when the channel is at capacity, the oldest value [is dropped]" and the slow receiver gets `RecvError::Lagged`, which "does **not** close or disconnect the receiver" [S-TOKIO-BCAST]. That's workable: map both `Ok` and `Lagged` to a `changed` event (`tokio_stream::wrappers::BroadcastStream` yields `BroadcastStreamRecvError::Lagged`) [S-TOKIO-STREAM]. But it's a buffer you don't need.
- **watch (recommended):** it holds only the latest value. `changed()` returns once there is an unseen value and marks it seen [S-TOKIO-WATCH]. Keep a `watch::Sender<u64>` log version and `send_modify(|v| *v += 1)` after each committed write. Each SSE stream is a `WatchStream::from_changes(rx)` mapped to `Event::default().event("changed").data(v.to_string())`. Bursts coalesce, nothing can lag, and the version number is handy when debugging *(inference)*.
- Send only *after* the transaction commits, so a refetch never sees uncommitted state *(inference)*.

## htmx 2 integration

- Vendor `htmx.min.js` **2.0.11** (npm `latest`, while 4.0.0 is on `next`) and `htmx-ext-sse` **2.2.4** [S-NPM-HTMX][S-HTMX-SSE]. In htmx 2 the extension is a separate file loaded after core, enabled with `hx-ext="sse"`, opened with `sse-connect="/events"`. `hx-trigger="sse:changed"` plus `hx-get` refetches a fragment. It "adds its own reconnection logic (using an exponential-backoff algorithm)" and fires `htmx:sseOpen` and `htmx:sseError` [S-HTMX-SSE].
- Pattern: `<div id="pots" hx-get="/fragments/pots" hx-trigger="sse:changed, refresh from:body">`, where the askama `blocks = ["pots", "log"]` struct serves both the full page and `/fragments/*` *(inference)*.
- **iPad wake:** add about 10 lines that, on `visibilitychange` to visible or on `pageshow`, trigger `refresh` on the body (and, if `htmx:sseError` fired or the source is closed, reload the `sse-connect` element) [S-HTMX-SSE] *(inference; same rule as ticket 01)*.
- **Never serve a non-200 or a non-`text/event-stream` response on `/events`.** `EventSource` "does not attempt to reconnect" after that [S-WHATWG-SSE]. With axum the only risk is a 403 from the CSRF layer, and that layer passes GETs (rule 1), or a 404 from a typo *(inference)*.

## Time zone for "once per kid per day"

The local date decides the daily-chore limit. jiff 0.2's default features read the system tzdb (`tz-system`, `tzdb-zoneinfo`). The `tzdb-bundle-always` feature embeds the tz database via `jiff-tzdb` [S-CRATES]. Enable it and resolve a configured zone (env `TZ` or `PEBBLEPOT_TZ`) with `jiff::tz::TimeZone::get`, so the image needs no `tzdata` package *(inference)*. Including `pkgs.tzdata` and setting `TZ` also works, as in ticket 01.

## Nix packaging: crane + dockerTools (runner-up buildRustPackage)

- **crane**, v0.24.0 (2026-08-21), active: "build your workspace dependencies just once, then quickly lint, build, and test changes to your project" [S-CRANE]. Use `cargoArtifacts = craneLib.buildDepsOnly commonArgs;` then `craneLib.buildPackage (commonArgs // { inherit cargoArtifacts; })` [S-CRANE-QS]. Dependencies are cached in their own derivation, so editing a template doesn't rebuild tokio and friends *(inference)*.
- **Crane source-filter gotcha:** `cleanCargoSource`/`filterCargoSources` "will only keep files whose names end with `.rs` or `.toml`" [S-CRANE-SRC]. Askama templates (`templates/*.html`), `include_str!` migrations (`migrations/*.sql`) and embedded `static/*.js|css` would all vanish and break the build. Use the fileset form: `lib.fileset.unions [ (craneLib.fileset.commonCargoSources ./.) ./templates ./migrations ./static ]` [S-CRANE-SRC].
- **nixpkgs `rustPlatform.buildRustPackage`** is the zero-input alternative. It needs either `cargoHash` (which "requires that the hash is updated after every change to `Cargo.lock`") or `cargoLock.lockFile = ./Cargo.lock`, which avoids that [S-NIX-RUST]. It rebuilds every dependency on each change. Fine for a deployment build, slower when iterating *(inference)*. The nixpkgs manual itself points to crane and naersk as external options [S-NIX-RUST]. **naersk** has no tagged releases and its last commit was 2026-06-23 [S-CRANE-GH]. Not recommended.
- **Bundled SQLite in Nix:** `libsqlite3-sys` with `bundled` compiles the amalgamation through the `cc` build dependency [S-RQ-SYS]. The stdenv C compiler is enough, so add no `buildInputs` (and no `pkg-config`/`sqlite`) *(inference)*.
- **Static/musl is unnecessary.** `dockerTools.streamLayeredImage` includes the closure automatically ("The closure of `config` is automatically included in the generated image") [S-NIX-DT], so the glibc-linked binary carries its glibc into the image. musl only matters for copying the binary outside Nix. crane ships a `cross-musl` example if that's ever needed [S-CRANE-GH] *(inference)*.
- **Recipe shape** *(inference)*:
  `pebblepot = craneLib.buildPackage { src = <fileset>; cargoArtifacts = …; strictDeps = true; };`
  `image = dockerTools.streamLayeredImage { name = "pebblepot"; tag = "latest"; config = { Cmd = [ "${pebblepot}/bin/pebblepot" ]; Env = [ "PEBBLEPOT_DB=/data/pebblepot.db" "PEBBLEPOT_TZ=…" "PORT=8080" ]; ExposedPorts."8080/tcp" = {}; Volumes."/data" = {}; }; };`
  `virtualisation.oci-containers.containers.pebblepot.imageStream = image;` with `backend = "docker"`, as in ticket 01.
- **Toolchain:** nixos-26.05's `rustc` is 1.95.0, so no rust-overlay or fenix is needed for these MSRVs (askama 1.88, tokio 1.71, sqlx 1.94) [S-NIX-ALLPKGS][S-TOKIO][S-CRATES].
- **Signals in the container:** the binary is PID 1 when `Cmd` points straight at it. Tokio's `tokio::signal::unix::signal(SignalKind::terminate())` handles the SIGTERM from `docker stop`. Install it explicitly, because PID 1 gets no default SIGTERM action from the kernel *(inference; Linux PID-1 semantics)*.

## Cross-stack pitfalls carried over from ticket 01

| Pitfall | How the Rust set handles it |
|---|---|
| `docker stop` sends SIGKILL after about 10 s | `CancellationToken` → `with_graceful_shutdown` + `take_until` on every SSE stream. axum 0.8 otherwise waits forever [S-AXUM-SERVE-089] |
| iPad SSE dies after sleep | htmx sse backoff, plus a `visibilitychange`/`pageshow` refetch and reconnect. Keep-alive 15 s by default in axum |
| `EventSource` gives up for good on non-200 | `/events` is a plain GET and `CsrfLayer` passes GETs |
| WAL backups | rusqlite `backup` feature or `VACUUM INTO`. Local disk only |
| Plain HTTP | `CsrfLayer`'s `Origin`/`Host` fallback needs no configuration over HTTP. No `Sec-Fetch-Site` arrives on HTTP [S-FETCH-META] |
| Local day | jiff with a bundled tzdb, or `tzdata` + `TZ` |

## Sources

- [S-CRATES] crates.io API, `https://crates.io/api/v1/crates/<name>` (versions, dates, features, dependencies), queried 2026-10-09
- [S-AXUM-SSE] axum source, `axum/src/response/sse.rs`: https://github.com/tokio-rs/axum/blob/main/axum/src/response/sse.rs
- [S-AXUM-SERVE-089] axum 0.8.9 source, `axum/src/serve/mod.rs`: https://github.com/tokio-rs/axum/blob/axum-v0.8.9/axum/src/serve/mod.rs
- [S-AXUM-CL] axum CHANGELOG (Unreleased section): https://github.com/tokio-rs/axum/blob/main/axum/CHANGELOG.md
- [S-TOKIO] tokio README, LTS releases: https://github.com/tokio-rs/tokio/blob/master/README.md
- [S-TOKIO-BCAST] tokio 1.53.2 source, `tokio/src/sync/broadcast.rs`: https://github.com/tokio-rs/tokio/blob/tokio-1.53.2/tokio/src/sync/broadcast.rs
- [S-TOKIO-WATCH] tokio 1.53.2 source, `tokio/src/sync/watch.rs`: https://github.com/tokio-rs/tokio/blob/tokio-1.53.2/tokio/src/sync/watch.rs
- [S-TOKIO-STREAM] tokio-stream source, `src/wrappers/broadcast.rs` and `Cargo.toml`: https://github.com/tokio-rs/tokio/tree/tokio-1.53.2/tokio-stream
- [S-THTTP-CL] tower-http CHANGELOG (0.7.0, 0.7.1): https://github.com/tower-rs/tower-http/blob/main/tower-http/CHANGELOG.md
- [S-THTTP-CSRF] tower-http 0.7.1 source, `src/csrf/mod.rs`: https://github.com/tower-rs/tower-http/blob/tower-http-0.7.1/tower-http/src/csrf/mod.rs
- [S-THTTP-CSRF-SVC] tower-http 0.7.1 source, `src/csrf/service.rs`: https://github.com/tower-rs/tower-http/blob/tower-http-0.7.1/tower-http/src/csrf/service.rs
- [S-FETCH-META] W3C Fetch Metadata Request Headers, §3: https://www.w3.org/TR/fetch-metadata/
- [S-ACTIX-SERVER] actix-web source, `actix-web/src/server.rs` (`shutdown_timeout`): https://github.com/actix/actix-web/blob/master/actix-web/src/server.rs
- [S-AWL] actix-web-lab README: https://github.com/robjtede/actix-web-lab/blob/main/actix-web-lab/README.md
- [S-ASKAMA] askama README: https://github.com/askama-rs/askama
- [S-ASKAMA-BOOK] askama book, creating templates (`block`, `blocks`): https://askama.rs/en/stable/creating_templates.html (source `book/src/creating_templates.md`)
- [S-ASKAMA-FW] askama book, working with web frameworks: https://askama.rs/en/stable/frameworks.html (source `book/src/frameworks.md`)
- [S-RQ-TOML] rusqlite 0.40.2 `Cargo.toml` features: https://github.com/rusqlite/rusqlite/blob/v0.40.2/Cargo.toml
- [S-RQ-H] rusqlite 0.40.2 bundled `libsqlite3-sys/sqlite3/sqlite3.h` (`SQLITE_VERSION "3.53.2"`)
- [S-RQ-SYS] libsqlite3-sys 0.38.2 `Cargo.toml` (`links = "sqlite3"`, `cc` build-dep): https://github.com/rusqlite/rusqlite/blob/v0.40.2/libsqlite3-sys/Cargo.toml
- [S-RQ-BACKUP] rusqlite source, `src/backup.rs`: https://github.com/rusqlite/rusqlite/blob/v0.40.2/src/backup.rs
- [S-RQ-TX] rusqlite source, `src/transaction.rs`: https://github.com/rusqlite/rusqlite/blob/v0.40.2/src/transaction.rs
- [S-RQM] rusqlite_migration README: https://github.com/cljoly/rusqlite_migration
- [S-TRQ] tokio-rusqlite README: https://github.com/programatik29/tokio-rusqlite
- [S-SQLX-CL] sqlx CHANGELOG (0.9.0): https://github.com/launchbadge/sqlx/blob/v0.9.0/CHANGELOG.md
- [S-SQLX-OPT] sqlx 0.9.0 source, `sqlx-sqlite/src/options/mod.rs`: https://github.com/launchbadge/sqlx/blob/v0.9.0/sqlx-sqlite/src/options/mod.rs
- [S-SQLX-LIB] sqlx 0.9.0 source, `sqlx-sqlite/src/lib.rs`: https://github.com/launchbadge/sqlx/blob/v0.9.0/sqlx-sqlite/src/lib.rs
- [S-SQLX-TOML] sqlx 0.9.0 `sqlx-sqlite/Cargo.toml` (libsqlite3-sys range): https://github.com/launchbadge/sqlx/blob/v0.9.0/sqlx-sqlite/Cargo.toml
- [S-SQLX-FAQ] sqlx FAQ (offline mode): https://github.com/launchbadge/sqlx/blob/v0.9.0/FAQ.md
- [S-SQLITE-WAL] SQLite, write-ahead logging: https://www.sqlite.org/wal.html
- [S-WHATWG-SSE] HTML Living Standard, server-sent events: https://html.spec.whatwg.org/multipage/server-sent-events.html
- [S-HTMX-SSE] htmx SSE extension docs: https://htmx.org/extensions/sse/ and npm `htmx-ext-sse`
- [S-NPM-HTMX] npm registry, `htmx.org` dist-tags and times
- [S-CRANE] crane docs: https://crane.dev/
- [S-CRANE-QS] crane example, `examples/quick-start-simple/flake.nix`: https://github.com/ipetkov/crane/blob/master/examples/quick-start-simple/flake.nix
- [S-CRANE-SRC] crane docs, source filtering: https://github.com/ipetkov/crane/blob/master/docs/source-filtering.md
- [S-CRANE-SQLX] crane example, `examples/sqlx/flake.nix`: https://github.com/ipetkov/crane/blob/master/examples/sqlx/flake.nix
- [S-CRANE-GH] GitHub API, releases and commits for `ipetkov/crane` and `nix-community/naersk`; crane `examples/` listing
- [S-NIX-RUST] Nixpkgs manual source (nixos-26.05), `doc/languages-frameworks/rust.section.md`: https://github.com/NixOS/nixpkgs/blob/nixos-26.05/doc/languages-frameworks/rust.section.md
- [S-NIX-DT] Nixpkgs manual source (nixos-26.05), dockerTools: https://github.com/NixOS/nixpkgs/blob/nixos-26.05/doc/build-helpers/images/dockertools.section.md
- [S-NIX-ALLPKGS] nixpkgs `pkgs/top-level/all-packages.nix` and `pkgs/development/compilers/rust/1_*.nix` on `nixos-26.05` and `nixos-unstable`
