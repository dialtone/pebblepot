# HTTPS for the Pebblepot PWA on the home network

Question (ticket [02](../issues/02-https-for-pwa-at-home.md)): what is the simplest reliable way to serve Pebblepot over HTTPS from a NixOS home server, reached over the LAN and WireGuard but not exposed to the internet, so that iOS Safari and Android Chrome trust it, register the service worker, and install the PWA?

Researched 2026-10-07. Claims are cited inline. Anything marked **(inference)** is my reasoning from the cited sources, not a direct statement from them.

## 1. Why HTTPS is required at all

- Service workers need a secure context. MDN: "Only secure contexts are allowed to register service workers, so service workers are always secure contexts." [MDN: Secure contexts](https://developer.mozilla.org/en-US/docs/Web/Security/Secure_Contexts)
- The only plain-HTTP origins that count as "potentially trustworthy" are loopback ones (`127.0.0.0/8`, `::1`, `localhost`, `*.localhost`). `http://192.168.x.y` and `http://server.local` are not secure contexts. [MDN: Secure contexts](https://developer.mozilla.org/en-US/docs/Web/Security/Secure_Contexts) (the LAN-IP case is inferred from the listed rules)
- Chrome's install criteria start with "Be served over HTTPS", plus a manifest with `name`/`short_name`, 192px and 512px icons, `start_url`, and a `display` of `standalone`/`fullscreen`/`minimal-ui`/`window-controls-overlay`. [web.dev: install criteria](https://web.dev/articles/install-criteria)
- On iOS/iPadOS, users install from Share > "Add to Home Screen". A manifest with `display` set to `standalone` or `fullscreen` makes the saved icon open as an app, while without one it is saved as a bookmark. [WebKit blog: Web Push for Web Apps on iOS and iPadOS](https://webkit.org/blog/13878/web-push-for-web-apps-on-ios-and-ipados/)
  - Note: iOS will put a plain-HTTP site on the Home Screen. But without a secure context there is no service worker, so no offline support and no Web Push. "Installs" really means "installs with a working service worker". **(inference)**

## 2. The certificate must be *trusted*, not clicked through

- Chromium deliberately refuses to register a service worker whose script fetch hits a certificate error: "Failed to register a ServiceWorker: An SSL certificate error occurred when fetching the script." The "Proceed (unsafe)" interstitial exception does not cover this. [Chromium code review 643773004](https://codereview.chromium.org/643773004), [MSW docs: using local HTTPS](https://mswjs.io/docs/recipes/using-local-https)
- I found no first-party WebKit statement on service workers behind a clicked-through certificate. Treat a self-signed, untrusted certificate as non-working on iOS too, and don't rely on it. **(inference)**
- Apple's TLS requirements (iOS 13+): RSA keys of at least 2048 bits, a SHA-2 signature, the DNS name in the SAN (CN is ignored), EKU `id-kp-serverAuth`, and validity of 825 days or less. Apple says connections that violate these "will fail". [Apple HT210176](https://support.apple.com/en-us/HT210176). Self-made certificates (`openssl req -x509 -days 3650`, missing SAN or EKU) often break these rules. **(inference)**

## 3. Options

### Option A: a real domain plus a Let's Encrypt certificate via DNS-01 (recommended)

- DNS-01 "can validate domain names whose webservers aren't exposed to the public internet" and supports wildcards. HTTP-01 needs port 80 reachable from the internet, so it doesn't fit here. [Let's Encrypt: challenge types](https://letsencrypt.org/docs/challenge-types/)
- Requirements: you own a domain, and its DNS provider has an API. Let's Encrypt lists "Your DNS provider might not offer an API" as a con, and warns that DNS API credentials on the server raise the impact of a compromise, so use narrowly scoped tokens. [Let's Encrypt: challenge types](https://letsencrypt.org/docs/challenge-types/)
- Trust: Let's Encrypt chains to roots already shipped on iOS and Android, so devices need no setup, and Apple's TLS requirements are met by a public CA certificate.
- Name resolution: clients need `pebblepot.home.example.com` to resolve to the server's LAN IP. There are two ways to do this:
  1. A public A record pointing at the private LAN IP. This is simplest, and works on LAN and over WireGuard as long as WireGuard routes the LAN subnet. Caveat: routers or resolvers with DNS-rebinding protection (e.g. dnsmasq `stop-dns-rebind`) drop public answers that point at private IPs, so you may need to whitelist the domain. **(general knowledge, not from a cited source)**
  2. A local DNS override on the router, Pi-hole, or the server, also pushed as the WireGuard `DNS =`.
- The certificate only says the name is yours. The server still isn't reachable from the internet, so nothing is exposed.

**Option A1: NixOS `security.acme` (lego) plus Caddy or nginx using the certificate.** No custom build is needed. This is the most NixOS-native route.

From the NixOS manual's ACME chapter ([source](https://github.com/NixOS/nixpkgs/blob/master/nixos/modules/security/acme/default.md)):
- You must set `security.acme.acceptTerms = true;` and an email (`security.acme.defaults.email`).
- DNS-01 is configured per cert with `dnsProvider` (a lego provider code), plus `environmentFile` or `credentialFiles` for secrets. "ACME servers will only hand out wildcard certs over DNS validation."
- `useACMEHost` lets a vhost reuse a cert. `group` must let the web server read it.
- Option descriptions come from [acme/default.nix](https://github.com/NixOS/nixpkgs/blob/master/nixos/modules/security/acme/default.nix): `dnsProvider`, `environmentFile`, `credentialFiles` ("Environment variables suffixed by "_FILE" or "_PATH""), `dnsResolver`, `dnsPropagationCheck`, `extraDomainNames`, `group`, `reloadServices`.
- lego supports about 220 DNS providers, including Cloudflare, Route 53, Porkbun, deSEC, Duck DNS, Namecheap and Gandi. [lego DNS providers](https://go-acme.github.io/lego/dns/)
- The NixOS Caddy module wires `virtualHosts.<name>.useACMEHost` itself. It emits `tls <certdir>/cert.pem <certdir>/key.pem`, sets the cert's `group` to Caddy's group, adds `reloadServices = [ "caddy.service" ]`, and orders `caddy.service` after `acme-<name>.service`. [caddy/default.nix](https://github.com/NixOS/nixpkgs/blob/master/nixos/modules/services/web-servers/caddy/default.nix)

Sketch (not applied; `example.com`, the provider, and the container port are placeholders):

```nix
security.acme = {
  acceptTerms = true;
  defaults.email = "you@example.com";
  certs."home.example.com" = {
    domain = "*.home.example.com";        # or just "pebblepot.home.example.com"
    dnsProvider = "cloudflare";            # lego provider code
    environmentFile = "/var/lib/secrets/acme-dns.env";  # e.g. CF_DNS_API_TOKEN=..., kept out of the Nix store
  };
};

services.caddy = {
  enable = true;
  virtualHosts."pebblepot.home.example.com" = {
    useACMEHost = "home.example.com";
    extraConfig = ''
      reverse_proxy 127.0.0.1:8080   # Pebblepot container published on localhost
    '';
  };
};

networking.firewall.allowedTCPPorts = [ 443 ];  # LAN/WireGuard only; no router port-forward
```

(nginx works the same way: `services.nginx.virtualHosts.<name>.useACMEHost` plus `forceSSL`, as shown in the NixOS manual ACME chapter.)

**Option A2: Caddy obtaining the certificate itself via DNS-01.** This works, but Caddy needs a build with a `caddy-dns/<provider>` plugin: "a Caddy build with a caddy-dns module plugged in for your DNS provider". [Caddy: Automatic HTTPS](https://caddyserver.com/docs/automatic-https), [Caddy: tls directive](https://caddyserver.com/docs/caddyfile/directives/tls) (`tls { dns cloudflare {env.CLOUDFLARE_API_TOKEN} }`). On NixOS that means `services.caddy.package = pkgs.caddy.withPlugins { plugins = [ "github.com/caddy-dns/cloudflare@<tag>" ]; hash = "<fixed-output hash>"; };`, where plugins must carry a tag or pseudo-version ([plugins.nix](https://github.com/NixOS/nixpkgs/blob/master/pkgs/by-name/ca/caddy/plugins.nix)), plus `services.caddy.environmentFile` for the token. This is more moving parts than A1 for no gain. Traefik or Caddy running in Docker with a DNS plugin is equivalent, but moves certificate state into a container volume.

### Option B: a private CA installed on every device (fallback if no domain)

- Caddy's `tls internal` issues certificates from its own local CA. Its root and intermediate live under `pki/authorities/local` in Caddy's data directory (in the official Docker image, `/data/caddy/pki/authorities/local/root.crt`). Distributing the root to other devices is the admin's job, and automatic trust install "isn't guaranteed to work, especially if containers are being used". [Caddy: Automatic HTTPS](https://caddyserver.com/docs/automatic-https), [Caddy: tls directive](https://caddyserver.com/docs/caddyfile/directives/tls). Let's Encrypt also points to this approach (self-signed or a local root, e.g. mkcert, added to trust stores). [Let's Encrypt: certificates for localhost](https://letsencrypt.org/docs/certificates-for-localhost/)
- **iOS/iPadOS:** install the root as a profile, then go to Settings > General > About > Certificate Trust Settings and enable "full trust for root certificates". Without that second step, the root isn't trusted for TLS. [Apple 102390](https://support.apple.com/en-us/102390). Leaf certificates must still meet the 825-day rule in HT210176. Caddy's short-lived auto-renewed leaves do. **(inference)**
- **Android:** go to Settings > Security & privacy > More security settings > Encryption & credentials > Install a certificate > CA certificate. (The Pixel help page describes the same flow for "Wi-Fi certificate"; menu names vary by vendor and version.) [Google Pixel help](https://support.google.com/pixelphone/answer/2844832). Since Android 7, *apps* ignore user CAs unless they opt in ([Android network security config](https://developer.android.com/privacy-and-security/security-config)). Chrome's own verifier, though, loads user-added roots as trust anchors (`net::android::GetUserAddedRoots()` → `AddTrustAnchor`) with no feature-flag gate in that file. [Chromium trust_store_android.cc](https://chromium.googlesource.com/chromium/src.git/+/main/net/cert/internal/trust_store_android.cc). So Chrome should trust it, but I found no first-party Chrome doc stating this, so test on a real device before committing to this route.
- NixOS config: `services.caddy.enable = true; virtualHosts."pebblepot.lan".extraConfig = "tls internal\nreverse_proxy 127.0.0.1:8080";` plus local DNS for the chosen name. Root cert: `/var/lib/caddy/.local/share/caddy/pki/authorities/local/root.crt` under the NixOS module's data dir (path is an **inference** from Caddy's XDG data-dir default; verify on the box).
- Costs: a manual step on every phone and tablet (and again after factory resets or new devices), the iOS full-trust toggle is easy to miss, and a leaked root key could MITM every enrolled device.

### Option C: a self-signed leaf certificate (not viable)

- Not trusted by any device, so the browser shows an interstitial. Chromium refuses to register the service worker after a click-through (section 2), so there is no offline PWA. Installing the self-signed leaf itself as a trusted root on each device is just a worse version of Option B, with long-lived leaves that may break Apple's 825-day/EKU/SAN rules.

## 4. Summary table

| Option | Device setup | iOS SW and install | Android Chrome SW and install | NixOS pieces | Prerequisites |
|---|---|---|---|---|---|
| A1: domain, DNS-01 via `security.acme` plus Caddy `useACMEHost` | none | yes | yes | `security.acme.certs.*` (dnsProvider, environmentFile), `services.caddy.virtualHosts.*.useACMEHost`, firewall 443 | own a domain whose DNS provider has an API (lego-supported), a scoped API token, DNS name to LAN IP |
| A2: domain, Caddy DNS plugin | none | yes | yes | `pkgs.caddy.withPlugins`, `services.caddy.environmentFile`, `tls { dns … }` | same as A1 |
| B: Caddy `tls internal` / private CA | install root on every device, and enable full trust on iOS | yes, once trusted | yes, once installed (verify on device) | `services.caddy` with `tls internal`, local DNS | none external |
| C: self-signed | click-through | no service worker | no (Chromium blocks SW) | n/a | n/a |

## 5. Recommendation

Use **A1**: a subdomain of a domain the household owns, a Let's Encrypt certificate via DNS-01 using NixOS `security.acme`, and Caddy on the host terminating TLS and reverse-proxying to the Pebblepot container on localhost. No device setup is needed, it's trusted everywhere, renewal is a systemd timer, and nothing is exposed to the internet. If the household won't buy a domain, use **B** and accept the per-device root install.

## Open points

- Whether the home router or resolver blocks public DNS answers pointing at private IPs (rebind protection). Check when deploying.
- Android Chrome trusting user CAs (Option B only) is backed by Chromium source but not by a doc, so verify on a device.
