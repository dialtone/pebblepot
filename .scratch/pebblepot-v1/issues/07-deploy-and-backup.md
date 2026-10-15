# How is Pebblepot deployed, updated and backed up on the NixOS server?

Type: grilling
Status: resolved
Blocked by: 01, 05

## Question

The stack is settled in [Which stack should run Pebblepot on the NixOS home server?](01-stack-on-home-server.md) and [Which Rust libraries should Pebblepot use?](05-rust-crates.md): Rust + htmx, built with crane into a Nix Docker image and run with `oci-containers` on the Docker backend. That leaves these questions:
- **Configuration:** where does the NixOS config for Pebblepot live? In the app repo as a flake module the server imports, or in the server's own config?
- **Updates:** how does a new build reach the server? For example a `nixos-rebuild` that pulls a pinned flake input, or something else.
- **Network:** which port and hostname does it use on the LAN and WireGuard?
- **Backups:** where does the SQLite file live, and how is it backed up (with `VACUUM INTO` or SQLite's online backup, never a raw file copy)? How often, where do backups go, and how long are they kept?
- **Restore:** how is a backup restored, and has a restore been tested?
- **Monitoring:** does anything need to watch that it's up?

Fit the answers to the server's existing setup: the apps already running there and any backup tooling already in place.

## Answer

- **Packaging.** The `dialtone/pebblepot` repo will be a **public** GitHub repo exporting a flake. The flake holds the app, a Nix-built Docker image (crane + `dockerTools`, see [Which Rust libraries should Pebblepot use?](05-rust-crates.md)) and a NixOS module. The `dialtone/nixos` flake adds `pebblepot` as an input and imports the module into `modules/homelab/`. There's no registry: the image loads into Docker through `imageStream`/`imageFile`.
- **The module** copies the Hermes pattern:
  - `virtualisation.oci-containers.containers.pebblepot` on the Docker backend
  - the port published on loopback only, `127.0.0.1:8070:8070`
  - data in `${vars.serviceConfigRoot}/pebblepot` (`/persist/opt/services/pebblepot`), on local disk as WAL mode requires
- **Network.** Add `pebblepot.port = 8070` to the `apps` attrset in `traefik.nix`, giving **http://pebblepot.dabass** behind `local-ip-whitelist`. `*.dabass` resolves on the LAN and over the router's WireGuard, as the user confirmed.
- **Updates are manual:** `nix flake update pebblepot` in the nixos repo, then `nixos-rebuild switch`. The version is pinned by `flake.lock`.
- **Backups:**
  - a nightly systemd timer runs SQLite's `VACUUM INTO` to write a consistent copy into a Pebblepot folder on `/mnt/storage`, which SnapRAID protects
  - it keeps 30 daily and 12 monthly copies
  - it never copies the raw `.db`
- **Restore** is a **script shipped by the module** (for example `pebblepot-restore <backup-file>`). It stops `docker-pebblepot`, copies the backup over the database, removes any `-wal`/`-shm` files and starts the service again. It is tested once at go-live.
- **Alerting:** `OnFailure=` on `docker-pebblepot.service` and on the backup service calls the existing `notify` script, the same way SnapRAID reports.
- **Out of scope for v1:** off-box backup (for example restic to cloud storage).

## Comments

**2026-10-15, user input (verbatim):** "happy to do a docker container here this is my https://github.com/dialtone/nixos nixos configuration"

**2026-10-15, facts read from github.com/dialtone/nixos** (shallow clone, read only):
- **Flake:** host `dabass`, a flake on `nixos-26.05`. Homelab apps are modules under `modules/homelab/`, imported from `modules/homelab/default.nix`.
- **Docker convention already in use:** `modules/homelab/hermes.nix` sets `virtualisation.docker.enable = true` and `virtualisation.oci-containers.backend = "docker"`, and runs `virtualisation.oci-containers.containers.hermes`. A seed `oneshot` service runs before `docker-hermes.service`. Pebblepot can copy this pattern.
- **Reverse proxy:** `modules/homelab/traefik.nix` runs Traefik on `:80` (plain HTTP). Each app gets a router `Host(\`<name>.dabass\`)` with the `local-ip-whitelist` middleware (127/8, 10/8, 172.16/12, 192.168/16), plus a loadBalancer to `http://localhost:<port>`. New apps are added to its `apps` attrset, so Pebblepot would be `pebblepot.port = <n>` → **http://pebblepot.dabass**. The whitelist already covers the LAN (192.168.88.0/24). Check that WireGuard clients' addresses are also covered.
- **App data:** goes in `vars.serviceConfigRoot = "/persist/opt/services"`, so Pebblepot's would be `/persist/opt/services/pebblepot`.
- **ZFS snapshots:** `configuration.nix` enables `services.zfs.autoSnapshot` on datasets with `com.sun:auto-snapshot=true`. A ZFS snapshot is atomic, so it's a crash-consistent copy of a live SQLite database in WAL mode *(inference: safe like a power cut, which SQLite survives)*.
- **Backups and alerts:** SnapRAID protects the data disks (`hosts/dabass/snapraid.nix`) and reports results through a `notify` script. No off-box backup tool (restic or borg) was found.

**2026-10-15, more facts from the config:**
- **ZFS auto-snapshots are switched off.** `services.zfs.autoSnapshot.enable = false` in `configuration.nix` (with `monthly = 48`), so nothing is snapshotting today.
- **`hosts/dabass/wireguard.nix` looks like an outbound Mullvad tunnel** (`address 10.64.245.75/32`, `allowedIPs 0.0.0.0/0`), not the remote-access WireGuard. Remote access is probably on the router, which fits the LAN being `192.168.88.0/24` *(inference)*. Traefik's whitelist allows 10/8, 172.16/12 and 192.168/16, so router-side WireGuard clients should get through. Remote devices also need DNS that resolves `*.dabass`.
- **Hermes publishes ports on loopback only** (`127.0.0.1:9119:9119`) and Traefik proxies to `localhost:<port>`. Pebblepot should do the same.

**2026-10-15, grilling answers:**
- Q1: yes, flake.
- Q2: public repo.
- Q3: manual updates.
- Q4: yes, pebblepot.dabass on 8070.
- Q5: (a) nightly `VACUUM INTO` to /mnt/storage.
- Q6: test the restore, and write a restore script.
- Q7: notify-based alerting is fine.
- Q8: `*.dabass` resolves over WireGuard too.

**2026-10-15, packaging revised (user's choice).** This replaces the "Packaging" and "The module" bullets in the Answer above:
- **Inline module.** The NixOS module lives **in the `dialtone/nixos` repo** as `modules/homelab/pebblepot.nix`, written in the same style as `hermes.nix`. The Pebblepot repo has no NixOS module.
- **The image comes from a plain Dockerfile** in the Pebblepot repo, built in two stages:
  1. compile with the official `rust` image
  2. copy the binary plus `templates/`, `migrations/` and `static/` into `debian:trixie-slim`

  There's no crane and no `dockerTools`.
- **Built on dabass:**
  - The nixos flake pins the Pebblepot repo as a source-only input (`flake = false`).
  - A oneshot `pebblepot-build` service, `requiredBy`/`before` `docker-pebblepot.service`, runs `docker build` on that source and tags the image `pebblepot:<shortRev>`. It skips the build if the tag already exists.
  - The container uses that local tag and never pulls from a registry.
  - The first start after an update compiles Rust on the server, which takes a few minutes and downloads crates. That's accepted.
- **Unchanged:**
  - loopback port 8070, data in `/persist/opt/services/pebblepot`, and http://pebblepot.dabass via Traefik
  - manual updates (`nix flake update pebblepot` + `nixos-rebuild switch`)
  - the nightly `VACUUM INTO` backup, the restore script (now shipped by the inline module) and `notify` alerting
- **Rejected:** a GitHub Actions build pushing to GHCR (the literal Hermes pattern), because it would need CI and a registry.
