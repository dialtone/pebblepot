# Which Rust libraries should Pebblepot use?

Type: research
Status: resolved
Blocked by: 01

## Question

Pebblepot is a Rust + htmx web app. The architecture is decided in [Which stack should run Pebblepot on the NixOS home server?](01-stack-on-home-server.md): plain HTTP, iPad-first, SQLite in WAL mode, SSE "changed" nudges that trigger refetches, and a Nix-built Docker image on x86-64 NixOS. Which libraries should it use for each of these?
- **Web framework**, e.g. axum or actix-web: SSE support, graceful shutdown that closes SSE streams promptly under `docker stop`, and serving static files.
- **HTML templating**, e.g. askama, maud, minijinja or rinja: compile-time checking, fit with htmx partials, and maintenance health.
- **SQLite access**, e.g. sqlx, rusqlite or diesel: WAL mode, migrations, transactions for the pot rules, async compatibility, and online backup or `VACUUM INTO`.
- **Live-update fan-out**, e.g. tokio broadcast.
- **Nix packaging**: `buildRustPackage`, crane or naersk, with `dockerTools`. Static or musl linking, and whether bundled SQLite complicates the build.

Recommend one combination with reasons and runner-ups, and use the maintenance and release status from primary sources.

## Answer

**Recommendation: axum + askama + rusqlite on tokio, packaged with crane.** One crate per concern:

- **Web framework:** `axum` 0.8.9 on `tokio` 1.53 (LTS until Sep 2027). SSE is built in (`Sse`, `Event`, `KeepAlive`, with a `:` keep-alive every 15 s by default).
- **Middleware and static files:** `tower-http` 0.7.1 with `csrf` (`CsrfLayer`, a port of Go's `CrossOriginProtection`) and, optionally, `fs` (`ServeDir`). Or embed the three assets with `include_bytes!`.
- **Shutdown:** `tokio-util` 0.7.20 `CancellationToken`. It drives `with_graceful_shutdown` and `take_until` on every SSE stream.
- **Templates:** `askama` 0.16.1 (compile-time checked) and `askama_web` 0.16 (`axum-0.8` feature). `#[template(blocks = ["pots", "log"])]` renders the htmx partials from the same page template.
- **SQLite:** `rusqlite` 0.40.2 with `bundled` and `backup` (SQLite 3.53.2), `rusqlite_migration` 2.6.0 (tracks the schema in `user_version`), and `tokio-rusqlite` 0.8.0. One connection on one thread handles everything, and every earn, redeem, undo and strike runs in a `transaction_with_behavior(Immediate)`.
- **Live-update fan-out:** a `tokio::sync::watch<u64>` log version, bumped after each commit and streamed through `tokio-stream` 0.1.19 `WatchStream` as `event: changed` with `data: <version>`. The data must be non-empty, because the SSE spec drops events whose data is empty.
- **Local day:** `jiff` 0.2.38 with `tzdb-bundle-always` and a configured zone. Or `tzdata` + `TZ`.
- **Front end:** vendored `htmx.min.js` 2.0.11 (npm `latest`; 4.0 is on `next`) and `htmx-ext-sse` 2.2.4. Use `hx-ext="sse"`, `sse-connect="/events"` and `hx-trigger="sse:changed"`, plus about 10 lines that refetch and reconnect on `visibilitychange`/`pageshow`.

**Runner-ups:**
- **actix-web 4.15.** It has a built-in `shutdown_timeout` force-drop (30 s by default), but its SSE responder is in the "experimental" `actix-web-lab` and tower-http's CSRF layer doesn't plug in.
- **sqlx 0.9.0.** It's async-native and checks SQL at compile time, but it needs a database or a `.sqlx` cache at build time, has no backup-API wrapper, its MSRV is 1.94, and it **can't share a binary with rusqlite 0.40** (`libsqlite3-sys <0.38` vs 0.38.2, `links = "sqlite3"`).
- **minijinja 3.0.0.** It has hot reload, but it checks templates at runtime and was released only yesterday.
- **maud 0.27.** No release in 20 months.
- **`tokio::sync::broadcast`.** It works, but you must treat `Lagged` as `changed`.
- **`rustPlatform.buildRustPackage` with `cargoLock.lockFile`.** It needs no flake input, but it rebuilds all dependencies on every change.
- **naersk.** No releases, and its last commit was in June 2026.

**Deciding reasons:**
1. axum has first-class SSE, it's the most widely used tokio-rs framework, and tower-http 0.7's `CsrfLayer` gives Go-equivalent CSRF protection with no tokens.
2. Over plain HTTP the CSRF layer needs no configuration. Browsers don't send `Sec-Fetch-Site` to non-trustworthy origins, so it falls back to a byte-exact, scheme-blind `Origin`/`Host` match, which works with any LAN or WireGuard hostname or IP.
3. askama's block rendering keeps the full page and the htmx fragments in one template, checked by the compiler.
4. rusqlite is the most direct SQLite binding: online backup, explicit `IMMEDIATE` transactions, and bundled SQLite. A single connection on its own thread serialises the pot rules without any `SQLITE_BUSY` handling.
5. `watch` coalesces bursts and can't lag, which is exactly what a "something changed, refetch" nudge needs.

**Gotchas for implementation:**
- **axum 0.8.9's graceful shutdown waits for in-flight requests with no timeout.** Without the cancellation token, open SSE streams hold the process until Docker's SIGKILL after about 10 s.
- **axum `main` already has breaking changes queued.** Expect one 0.9 migration.
- Askama has had a breaking minor about twice a year (0.13 to 0.16 since Mar 2025), and rusqlite one every 2 to 5 months. Both upgrades are small.
- The binary is PID 1 in the container, so install a SIGTERM handler explicitly (`tokio::signal::unix`).

**Nix/Docker packaging facts for the deployment fog:**
- **crane** v0.24.0 (2026-08-21, active): `cargoArtifacts = craneLib.buildDepsOnly commonArgs;` then `craneLib.buildPackage (commonArgs // { inherit cargoArtifacts; })`. Dependencies are cached in their own derivation.
- **crane's `cleanCargoSource` keeps only `*.rs` and `*.toml` files.** Use `lib.fileset.toSource` with `craneLib.fileset.commonCargoSources ./.` plus `./templates`, `./migrations` and `./static`, or askama templates, embedded SQL and htmx disappear from the build.
- Bundled SQLite compiles with the stdenv C compiler through `libsqlite3-sys`'s `cc` build dependency, so no `buildInputs`, `pkg-config` or system `sqlite` is needed.
- **No musl or static linking is needed.** `dockerTools.streamLayeredImage` puts the closure of `config` (including glibc) into the image automatically. Set `config.Cmd = [ "${pebblepot}/bin/pebblepot" ]`, `Env` (DB path `/data/pebblepot.db`, `PORT`, time zone), and `Volumes."/data"`. Then point `virtualisation.oci-containers.containers.pebblepot.imageStream` at the image, with `backend = "docker"` (see ticket 01).
- **Toolchain:** nixos-26.05's default `rustc` is 1.95.0, and nixos-unstable has 1.98.1. That covers every MSRV in the set (askama 1.88, sqlx 1.94 if chosen), so no rust-overlay or fenix is needed.
- If you use `buildRustPackage` instead, prefer `cargoLock.lockFile = ./Cargo.lock` over `cargoHash`, which has to change with every lockfile change.
- **Backups:** add a `pebblepot backup <path>` subcommand that uses rusqlite's online backup (or `VACUUM INTO`), run by a systemd timer through `docker exec`. Never copy the `.db` without its `-wal` file, and keep `/data` on local disk.

Full findings and sources: [research/rust-crates.md](../research/rust-crates.md)

## Comments

**2026-10-15, packaging superseded:** the crane + `dockerTools.streamLayeredImage` recipe is replaced by a plain two-stage Dockerfile (`rust` → `debian:trixie-slim`) built on dabass. See [How is Pebblepot deployed, updated and backed up on the NixOS server?](07-deploy-and-backup.md). The crate choices are unchanged. Bundled SQLite still compiles fine in the `rust` image.
