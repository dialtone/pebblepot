# 11: Production Dockerfile

**What to build:** A two-stage Dockerfile in the Pebblepot repo:
1. compile in the official `rust` image
2. copy the binary plus templates, migrations and static assets into `debian:trixie-slim`

The image listens on `0.0.0.0` inside the container and keeps its data in a mounted volume. It builds and runs on the developer's Apple Silicon Mac with Docker Desktop, and on x86-64 dabass.

**Blocked by:** 01

**Status:** ready-for-agent

Spec: [Pebblepot v1](../../pebblepot-v1/spec.md)

- [ ] `docker build` succeeds on an Apple Silicon Mac. `docker run` with a port mapping and a data-folder volume serves a working board.
- [ ] Data survives a container restart, because the SQLite file and its WAL live in the mounted volume.
- [ ] The container stops promptly on `docker stop`.
- [ ] No build stage needs anything beyond the two base images. Bundled SQLite compiles in the `rust` stage.
