# 11: Production Dockerfile

**What to build:** A two-stage Dockerfile in the Pebblepot repo:
1. compile in the official `rust` image
2. copy the binary plus templates, migrations and static assets into `debian:trixie-slim`

The image listens on `0.0.0.0` inside the container and keeps its data in a mounted volume. It builds and runs on the developer's Apple Silicon Mac with Docker Desktop, and on x86-64 dabass.

**Blocked by:** 01

**Status:** resolved

Spec: [Pebblepot v1](../../pebblepot-v1/spec.md)

- [x] `docker build` succeeds on an Apple Silicon Mac. `docker run` with a port mapping and a data-folder volume serves a working board.
- [x] Data survives a container restart, because the SQLite file and its WAL live in the mounted volume.
- [x] The container stops promptly on `docker stop`.
- [x] No build stage needs anything beyond the two base images. Bundled SQLite compiles in the `rust` stage.

## Comments

**2026-10-15, implemented** (PR build/ticket-11-dockerfile):
- **Image layout.** `Dockerfile` stage `build` on `rust:1.98-trixie` (pinned to the developer's stable toolchain; the lockfile needs ≥ 1.95 for `rusqlite_migration`) compiles `--release --locked`, dependencies first against stub sources so that layer is reused until `Cargo.toml`/`Cargo.lock` change. The run stage on `debian:trixie-slim` (same glibc as the builder) holds only `/usr/local/bin/pebblepot` and an empty `/data` owned by a fixed non-root user `pebblepot` **10001:10001**. Defaults: `PEBBLEPOT_LISTEN_ADDR=0.0.0.0`, `PEBBLEPOT_PORT=8070`, `PEBBLEPOT_DATA_DIR=/data`; `EXPOSE 8070`; exec-form `ENTRYPOINT`, so the app is PID 1 and gets SIGTERM directly. No `apt-get`, nothing beyond the two base images.
- **Departure from the ticket text:** templates, migrations and static assets are *not* copied into the run stage. They are already compiled into the binary (askama derives, `include_str!` for migrations and `static/`), and jiff bundles the tz database (`tzdb-bundle-always`), so copies on disk would never be read. `templates/`, `migrations/` and `static/` are copied into the **build** stage.
- **`.dockerignore`** drops `.git`, `target`, `data`, `.scratch`, `docs`, `tests`, `*.md`, so the build context is ~250 KB.
- **Build/run:** `docker build -t pebblepot .` then `docker run -d --name pebblepot -p 8070:8070 -v "$PWD/data:/data" -e PEBBLEPOT_TIME_ZONE=<IANA zone> pebblepot`. Without `PEBBLEPOT_TIME_ZONE` the container's zone (UTC) decides when "today" starts.
- **Size (estimate, not measured):** `debian:trixie-slim` is ~30 MB compressed (Docker Hub, amd64 and arm64) and the release binary is 5.3 MB (arm64 macOS build), so roughly 85 MB uncompressed. The `rust:1.98-trixie` builder pull is ~550–590 MB compressed, a one-time cost on dabass.
- **Not verified in Docker:** the Docker Desktop daemon wasn't running, so `docker build`/`docker run` were not executed and **no checklist box is ticked**; status left at `ready-for-agent`. Run the four checks (build on the Mac, board on a mapped port with a data volume, data after `docker restart`, prompt `docker stop`) and tick them.
- **Verified without Docker:** both base tags exist on Docker Hub for amd64 and arm64. Replaying the Dockerfile's two cargo steps in /tmp on the `.dockerignore`d context (stub build, then real build after `touch`) gives a binary with the templates and SQL embedded. Run alone from an empty directory with the image's env (port 8081, data under /tmp, `Europe/Rome`): board 200, the three static assets 200, a chore earn 200, a cross-origin POST 403; the SQLite file plus `-wal`/`-shm` appear in the data folder; SIGTERM with an open `/events` stream exits in 0.05 s and closes the stream; after a restart the earn is still there. `cargo test`: 77 pass.
- **For ticket 12 (NixOS module):**
  - The data folder `/persist/opt/services/pebblepot` must be writable by **10001:10001** (for example `systemd.tmpfiles` `d /persist/opt/services/pebblepot 0750 10001 10001 -`), or run the container with `user = "0:0"`.
  - The database is `pebblepot.sqlite3` in the data folder. The `-wal`/`-shm` files can stay after a stop (the app doesn't checkpoint on exit), so the restore script must remove them as planned, and `chown 10001:10001` the restored file.
  - The image has no `sqlite3` CLI. Run the backup's `VACUUM INTO` on the host with nixpkgs `sqlite` against the bind-mounted file (same kernel, local disk, so WAL sharing works), preferably as UID 10001 so any `-shm` it creates stays writable by the app.
  - Set `PEBBLEPOT_TIME_ZONE` in the container's `environment`; the other three already have image defaults but the spec wants all four explicit.
  - `docker stop`'s 10 s grace is ample: shutdown took 0.05 s with a live stream open.
  - `docker build` uses only the classic Dockerfile syntax (no `RUN --mount`), so it works with or without BuildKit.

**2026-10-17, verified in Docker** (PR build/ticket-11-verify-docker), Docker Desktop on Apple Silicon, from a cold image cache. No Dockerfile or code change was needed.
- **Build (arm64):** `docker build -t pebblepot:test .` exits 0 in **98 s** wall time, including the base-image pulls (~38 s). The dependency layer took 48 s and the app layer 8.5 s. The image is **107 MB** (101 MB is `debian:trixie-slim`, 6.7 MB is the binary), runs as user `pebblepot`.
- **Board and earn:** `docker run -d -p 127.0.0.1:8078:8070 -v <mktemp -d>:/data -e PEBBLEPOT_TIME_ZONE=America/Los_Angeles pebblepot:test`. `GET /` 200 (57 KB) and the static assets 200. A chore earn sent the way htmx in a browser on that host sends it (`Origin: http://localhost:8078`, `Sec-Fetch-Site: same-origin`, `HX-Request: true`) returns 200 and the row re-renders. The same POST with `Origin: http://evil.example`/`Sec-Fetch-Site: cross-site` returns 403.
- **Time zone:** the startup line reads `time zone America/Los_Angeles`. The earn made at `2026-10-18T05:06Z` was stored with `local_day = 2026-10-17`, so "today" follows `PEBBLEPOT_TIME_ZONE`, not the container's UTC. Without the variable the log says `Etc/UTC`. A plain `TZ=America/Los_Angeles` also works, because jiff reads it and the slim image has `/usr/share/zoneinfo`, but `PEBBLEPOT_TIME_ZONE` is the documented setting.
- **Persistence:** `pebblepot.sqlite3`, `-wal` and `-shm` appear in the mounted folder, and `journal_mode` is `wal`. The earn is still on the board and in the Today list after `docker restart`, and also after `docker rm` and a fresh `docker run` on the same folder.
- **Stop:** with an `/events` stream open, `docker stop` takes **0.14 s** and the exit code is 0. The stream closes cleanly (curl exits 0). The app logs nothing on shutdown, and the only log line is the startup one.
- **Base images only:** the Dockerfile has no `apt-get`, `curl` or `wget`. The `rust` stage downloads crates from crates.io during `cargo build`, which the deploy decision accepts, and the bundled SQLite compiles there with the image's own C compiler.
- **Data-folder ownership:** on Docker Desktop for macOS, a host folder with mode 700 owned by the Mac user works as is, because file sharing doesn't enforce the container's UID. Docker Desktop can't prove the Linux case with a bind mount, so it was reproduced with a tmpfs: a root-owned 755 `/data` makes the container exit 1 with `pebblepot: unable to open database file: /data/pebblepot.sqlite3`, and the same folder owned by 10001:10001 with mode 750 works. A named volume also works, because Docker copies the image's 10001-owned `/data` into an empty volume. The README now gives the Linux `chown` command.
- **amd64:** `docker build --platform linux/amd64` exits 0 in **127 s** (dependency layer 78 s, app layer 21 s) and the image is 86 MB. That image under emulation, with `Europe/Rome`, served the board (200), accepted the earn (200), refused the cross-site POST (403) and stopped in 0.14 s with exit code 0.
- **`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` and `cargo test`** are all clean.
- **For ticket 12:**
  - Create `/persist/opt/services/pebblepot` as **10001:10001**, for example with `systemd.tmpfiles` `d /persist/opt/services/pebblepot 0750 10001 10001 -`. Otherwise the container exits at once with `unable to open database file`.
  - Set `PEBBLEPOT_TIME_ZONE` explicitly.
  - Shutdown takes well under a second, so the default 10 s stop timeout is fine.
  - The first `pebblepot-build` on dabass should take a few minutes: the base pulls (~600 MB) plus about 1–2 min of compiling on native x86-64. Later builds that change only `src/` reuse the dependency layer (seconds).
