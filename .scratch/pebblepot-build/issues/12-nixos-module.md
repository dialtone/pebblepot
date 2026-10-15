# 12: NixOS module in dialtone/nixos

**What to build:** This ticket changes the **`dialtone/nixos`** repo, not Pebblepot. It adds a Hermes-style homelab module for Pebblepot:
- **Source:** the public `dialtone/pebblepot` repo pinned as a source-only flake input.
- **Build service:** a oneshot that runs `docker build` on that source before the container starts, tagging `pebblepot:<shortRev>` and skipping the build if the tag exists.
- **Container:** an `oci-containers` container (Docker backend) using that local tag with no pull. It publishes `127.0.0.1:8070`, its data lives under `/persist/opt/services/pebblepot`, and its listen address, port, data directory and time zone are set explicitly.
- **Proxy:** a `pebblepot` entry in Traefik's `apps` for http://pebblepot.dabass behind `local-ip-whitelist`.
- **Backup:** a nightly systemd timer runs SQLite `VACUUM INTO` to write a timestamped copy into a Pebblepot folder on the SnapRAID-protected main array, and prunes to 30 daily and 12 monthly copies.
- **Restore:** a `pebblepot-restore <backup-file>` command that stops the container, replaces the database, removes `-wal`/`-shm` files and starts the container again.
- **Alerts:** `OnFailure=` on the container and backup units calls the existing `notify` script.

**Blocked by:** 11

**Status:** ready-for-agent

Spec: [Pebblepot v1](../../pebblepot-v1/spec.md)

- [ ] The module is imported from the homelab module list, and `nixos-rebuild build` for host `dabass` succeeds.
- [ ] The build service is ordered before, and required by, the container unit, and skips work when the tag already exists.
- [ ] The container publishes on loopback only. Traefik routes `pebblepot.dabass` to it.
- [ ] The backup timer and its pruning are defined. The backup uses `VACUUM INTO`, never a raw file copy.
- [ ] `pebblepot-restore` is installed on the system path.
- [ ] Failure of the container or backup units triggers `notify`.
