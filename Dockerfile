# Pebblepot production image.
#
#   docker build -t pebblepot .
#   docker run -d --name pebblepot -p 8070:8070 -v "$PWD/data:/data" \
#     -e PEBBLEPOT_TIME_ZONE=Europe/Rome pebblepot
#
# Two stages, two base images, nothing else: the official `rust` image compiles the
# binary (bundled SQLite needs its C compiler), and `debian:trixie-slim` runs it.
# Templates (askama), migrations and static assets are compiled into the binary, and
# jiff bundles the time-zone database, so the runtime image needs only the binary.

# ---- build ----
# Pinned to the developer's stable toolchain; the lockfile needs Rust 1.95 or later.
FROM rust:1.98-trixie AS build
WORKDIR /src

# Build the dependencies against stub sources first, so this layer is reused
# until Cargo.toml or Cargo.lock changes.
COPY Cargo.toml Cargo.lock ./
RUN mkdir src \
 && echo 'fn main() {}' > src/main.rs \
 && touch src/lib.rs \
 && cargo build --release --locked \
 && rm -rf src

COPY src ./src
COPY migrations ./migrations
COPY static ./static
COPY templates ./templates
# COPY keeps the files' old mtimes; touch them so cargo rebuilds over the stubs.
RUN touch src/lib.rs src/main.rs \
 && cargo build --release --locked \
 && cp target/release/pebblepot /usr/local/bin/pebblepot

# ---- run ----
FROM debian:trixie-slim

# A fixed non-root user, 10001:10001. A bind-mounted data folder must be writable by it.
RUN groupadd --system --gid 10001 pebblepot \
 && useradd --system --uid 10001 --gid pebblepot --no-create-home \
      --home-dir /data --shell /usr/sbin/nologin pebblepot \
 && mkdir /data \
 && chown pebblepot:pebblepot /data

COPY --from=build /usr/local/bin/pebblepot /usr/local/bin/pebblepot

# Listen on every interface inside the container. The data folder is /data, a
# mounted volume that holds the SQLite file and its -wal/-shm files. Set
# PEBBLEPOT_TIME_ZONE to the household's IANA zone; without it the container's
# zone (UTC) decides when "today" starts.
ENV PEBBLEPOT_LISTEN_ADDR=0.0.0.0 \
    PEBBLEPOT_PORT=8070 \
    PEBBLEPOT_DATA_DIR=/data

USER pebblepot
WORKDIR /data
EXPOSE 8070

# Exec form: pebblepot is PID 1 and handles SIGTERM itself (graceful shutdown that
# also closes open live-update streams), so `docker stop` returns promptly.
ENTRYPOINT ["/usr/local/bin/pebblepot"]
