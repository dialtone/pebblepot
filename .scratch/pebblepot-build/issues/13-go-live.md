# 13: Go-live on dabass

**What to build:** A human puts Pebblepot live on dabass and proves the safety net works.

**Blocked by:** 12 (and ideally 01–10)

**Status:** ready-for-human

Spec: [Pebblepot v1](../../pebblepot-v1/spec.md)

- [ ] `nix flake update pebblepot` and `nixos-rebuild switch` on dabass succeed, and the first image build completes.
- [ ] http://pebblepot.dabass opens on the iPad (LAN) and on a phone over WireGuard, and a tap on one shows up on the other.
- [ ] One backup run produces a copy on the main array.
- [ ] `pebblepot-restore` with that copy brings the app back with the same pots.
- [ ] Stopping the container triggers a `notify` alert (or the alert path is otherwise checked).
