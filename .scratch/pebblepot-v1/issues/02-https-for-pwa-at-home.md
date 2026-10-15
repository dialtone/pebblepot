# How does Pebblepot get HTTPS on the home network so the PWA installs?

Type: research
Status: resolved
Blocked by:

## Question

Browsers only register service workers and offer "Add to Home Screen" install over a secure context. Pebblepot runs on a NixOS home server and is reached over the LAN and over WireGuard, not the public internet. What is the simplest reliable way to serve it over HTTPS that iOS Safari and Android Chrome will trust? Candidates include a real domain with a DNS-01 ACME certificate (e.g. Caddy or Traefik, or NixOS `security.acme`), a private CA installed on each device, and a self-signed certificate. Include which ones actually let iOS install the PWA, and what NixOS configuration each needs.

## Answer

**Recommendation:** Use a real domain and get a Let's Encrypt certificate via the **DNS-01** challenge with NixOS's built-in `security.acme` (lego). Caddy on the host terminates TLS with `useACMEHost` and reverse-proxies to the Pebblepot container on `127.0.0.1`. A DNS name such as `pebblepot.home.example.com` resolves to the server's LAN IP, which WireGuard clients reach through the LAN route. Nothing is port-forwarded or exposed to the internet.

**Why:** Service workers only register in a secure context, and Chromium refuses to register one behind an untrusted (clicked-through) certificate. So the certificate has to be genuinely trusted on every phone and tablet. DNS-01 works for servers that aren't publicly reachable, and Let's Encrypt is already trusted by iOS and Android, so devices need no setup. `security.acme` handles issuance and renewal as a systemd timer, and needs no custom Caddy build.

**Which options let iOS (and Android) install the PWA with a working service worker:**
- Domain + DNS-01 (via `security.acme`, or Caddy with a DNS plugin): **yes**, with no device setup.
- Private CA (Caddy `tls internal`, mkcert): **yes, but only after** installing the root on every device. On iOS you must also enable it under Settings > General > About > Certificate Trust Settings. Android Chrome loads user-added roots (per the Chromium source), but verify this on a device. This is the fallback if no domain is available.
- Self-signed certificate: **no**. It isn't trusted, so there is no service worker (and no offline PWA).

**NixOS config each needs (sketches, not applied):**
- Recommended: `security.acme.acceptTerms = true; defaults.email = …; certs."home.example.com" = { domain = "*.home.example.com"; dnsProvider = "<lego code>"; environmentFile = "/var/lib/secrets/acme-dns.env"; };` plus `services.caddy.virtualHosts."pebblepot.home.example.com" = { useACMEHost = "home.example.com"; extraConfig = "reverse_proxy 127.0.0.1:8080"; };` and open port 443 in `networking.firewall` (LAN/WireGuard only). The Caddy module sets the cert group, reload, and service ordering itself.
- Caddy doing DNS-01 itself: `services.caddy.package = pkgs.caddy.withPlugins { plugins = [ "github.com/caddy-dns/<provider>@<tag>" ]; hash = …; };`, `services.caddy.environmentFile`, and `tls { dns <provider> {env.TOKEN} }`.
- Private CA: `services.caddy` vhost with `tls internal`, local DNS for the name, and the root cert from Caddy's data dir (`pki/authorities/local/root.crt`) distributed to each device.

**Prerequisites the user must provide:**
- A domain they own, whose DNS provider has an API supported by lego (e.g. Cloudflare, Porkbun, deSEC, Route 53, Namecheap, Gandi), plus an API token scoped to DNS edits for that zone, stored outside the Nix store.
- A DNS record (public A record to the private LAN IP, or a local override on the router or Pi-hole that is also pushed as WireGuard `DNS =`) for the Pebblepot hostname. If using the public record, check that the router's DNS-rebind protection doesn't block it.

Research: [research/https-for-pwa-at-home.md](../research/https-for-pwa-at-home.md)


## Comments

**2026-10-07, ruled out of scope:** the user dropped the PWA for v1 because certificate setup is too much for now. Pebblepot v1 is an iPad-first responsive web app served over plain HTTP on the LAN and WireGuard. The answer above stays valid and is the route to take if the PWA is revisited.
