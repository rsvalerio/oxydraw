# Deploying oxydraw

Operations guide for self-hosting `oxydraw` — installing, configuring, and running it
in production. The reference target is a single Linux VM running the service under **systemd**,
installed from a **`.deb`** package, behind a TLS-terminating reverse proxy.

- New here? Skim [What you get](#what-you-get), then jump to
  [Install from the APT repository](#install-from-the-apt-repository).
- Just want to try it locally? See [Quick try](#quick-try).
- Full knob list: [Configuration reference](#configuration-reference).

## What you get

A single static-ish binary (`oxydraw`) that serves everything on one port:

- the **OxyDraw web UI** (a custom SPA on the `@excalidraw/excalidraw` editor, embedded in
  the binary at build time),
- **anonymous share links** (`/api/v2/post/`, `/api/v2/{id}`) and **scene image files**
  (`/api/files/{id}`) stored in SQLite,
- the **Socket.IO collaboration relay** (`/socket.io/`) plus **per-room scene snapshots**
  (`/api/rooms/{id}/scene`). Snapshots are held in process memory, capped at **256 MB**
  with LRU eviction — size the host's RAM accordingly.

Storage is `sqlite` (default, embedded) or `memory` (volatile, dev only). There are no
external service dependencies. The overlay's scene library can optionally be gated by a
shared password or by **Sign in with Google / GitHub** — see [AUTH.md](AUTH.md); the rest
of the server (share links, collab) stays unauthenticated by design.

## Install options

| Method | Best for | Notes |
| --- | --- | --- |
| **APT repository** | Debian/Ubuntu (amd64) + systemd | Easiest: `apt install oxydraw` with `apt`-managed updates. This guide's main path — see [Install from the APT repository](#install-from-the-apt-repository). |
| **Docker image** (GHCR) | Most self-hosters | Frontend is built & embedded for you. See [README](../README.md#release). |
| **`.deb` package** (build it) | Other arches / air-gapped | Build the package yourself, then install the file. Same hardened systemd unit — see [Build the `.deb` yourself](#install-on-a-vm-with-the-deb). |
| **Prebuilt binary** (cargo-dist) | Other Linux/macOS | Tarballs + shell installer attached to GitHub Releases. |
| **Homebrew** | macOS / Linuxbrew | `brew install rsvalerio/tap/oxydraw`. |
| **From source** | Development | `make release` (builds the frontend, then `cargo build --release` in `backend/`). |

All methods run the same binary and read the same [environment variables](#configuration-reference).

## Quick try

```bash
make run             # serves http://0.0.0.0:3002, SQLite at ./backend/oxydraw.db
```

---

## Install from the APT repository

The quickest path on Debian/Ubuntu: install the prebuilt **amd64** package from the `rsvalerio`
APT repo, then get updates through `apt` like any other package.

```bash
# 1. Trust the repository signing key.
sudo install -m 0755 -d /etc/apt/keyrings
curl -fsSL https://rsvalerio.github.io/apt/public.key \
  | sudo gpg --dearmor -o /etc/apt/keyrings/rsvalerio.gpg

# 2. Add the source (amd64 only).
echo "deb [arch=amd64 signed-by=/etc/apt/keyrings/rsvalerio.gpg] https://rsvalerio.github.io/apt stable main" \
  | sudo tee /etc/apt/sources.list.d/rsvalerio.list

# 3. Install.
sudo apt update
sudo apt install oxydraw
```

This installs exactly what the manual `.deb` does — the binary, the
`/etc/oxydraw/oxydraw.env` config (a conf-file, preserved across upgrades), and the
hardened systemd unit (**enabled but not started**). Now [configure](#3-configure) and
[start](#4-start-it) it.

Later releases arrive via `sudo apt update && sudo apt upgrade`: your env file is preserved and
the service is restarted automatically.

> Only `amd64` is published. On other architectures (e.g. arm64), or for an air-gapped install,
> [build the `.deb` yourself](#install-on-a-vm-with-the-deb).

---

## Install on a VM with the `.deb`

Prefer the [APT repository](#install-from-the-apt-repository) above when you can. Build the
package yourself when you need an architecture other than amd64, an air-gapped install, or a
locally patched build.

### 1. Build the package

The web UI is embedded into the binary **at compile time**, so it must be built *before* the
binary is compiled. Do this on a build machine matching your VM's architecture (e.g. an amd64
builder for an amd64 VM), with Rust ≥ 1.85 and Docker installed (the frontend builds inside the
`oven/bun` container; give the Docker VM ~4+ GB of memory — a 2 GB default OOMs the vite build).

```bash
# One-time: the .deb tooling
cargo install cargo-deb

# In a checkout of this repo:
make frontend                  # builds the UI (in Docker) into backend/crates/server/assets/

# Build the release binary (with the UI embedded) and package it (cargo deb runs in the workspace):
cd backend && cargo deb -p oxydraw   # compiles --release, then produces the .deb
# → backend/target/debian/oxydraw_<version>_<arch>.deb
```

> If you skip `make frontend`, the binary still builds but serves a placeholder page
> instead of the real UI. Always build the frontend first for a production package.

### 2. Install it

Copy the `.deb` to the VM and install — `apt` pulls in any runtime dependencies:

```bash
sudo apt install ./oxydraw_*.deb
```

This installs:

- `/usr/bin/oxydraw` — the binary,
- `/etc/oxydraw/oxydraw.env` — config (preserved across upgrades),
- the systemd unit `oxydraw.service` (**enabled but not started**),
- state dir `/var/lib/oxydraw/` (created by systemd on first start).

### 3. Configure

Edit `/etc/oxydraw/oxydraw.env` if needed:

```bash
sudoedit /etc/oxydraw/oxydraw.env   # storage path, host, LISTEN
```

The default env file uses SQLite at `/var/lib/oxydraw/oxydraw.db` and binds
`127.0.0.1:3002` (intended to sit behind a [reverse proxy](#reverse-proxy--tls)). The defaults
work as-is.

### 4. Start it

```bash
sudo systemctl start oxydraw
sudo systemctl status oxydraw        # should be active (running)
curl -sS http://127.0.0.1:3002/ | head     # the UI / placeholder HTML
```

The unit was already enabled by the package, so it starts on boot. Done — now put it behind
[TLS](#reverse-proxy--tls).

---

## Running under systemd

The bundled unit runs the service as a transient unprivileged user (`DynamicUser=yes`) with a
persistent `StateDirectory` at `/var/lib/oxydraw`, plus a tight sandbox (read-only system,
no home access, restricted syscalls). No `useradd` needed.

```bash
sudo systemctl start    oxydraw
sudo systemctl stop     oxydraw
sudo systemctl restart  oxydraw     # after editing the env file
sudo systemctl status   oxydraw
sudo systemctl enable   oxydraw     # start on boot (package does this already)

journalctl -u oxydraw -f            # follow logs
journalctl -u oxydraw -e            # jump to the end
```

Adjust log verbosity with `RUST_LOG` in the env file (e.g. `RUST_LOG=oxydraw=debug,info`),
then restart.

### Manual systemd install (no `.deb`)

Using a prebuilt binary or `cargo install` instead of the package? Replicate it by hand:

```bash
sudo install -m755 oxydraw /usr/bin/oxydraw
sudo install -d -m755 /etc/oxydraw
sudo install -m640 packaging/oxydraw.env /etc/oxydraw/oxydraw.env
sudo install -m644 packaging/systemd/oxydraw.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable --now oxydraw
```

---

## Configuration reference

All configuration is via environment variables (the `.deb` supplies them through the unit's
`EnvironmentFile`). A local non-systemd run can instead use a `.env` file — see
[`.env.example`](../.env.example).

| Variable | Default | Meaning |
| --- | --- | --- |
| `LISTEN` | `0.0.0.0:3002` | Bind address. Use `127.0.0.1:3002` behind a reverse proxy. |
<!-- the row below documents the insecure-default combination (SEC-29) -->
| _(open by default)_ | — | With no `EXT_PASSWORD` and no OAuth provider, the scene library and storage endpoints are unauthenticated; combined with the default `LISTEN=0.0.0.0:3002` they are reachable on **every** network interface. Fine for a trusted LAN or behind a loopback-bound reverse proxy — but if you run the binary unmodified on a host with a public interface, you expose an open, writable library to the internet. Set an auth method (see [AUTH.md](AUTH.md)) or bind `LISTEN=127.0.0.1:3002`. The server logs a startup **warning** when it detects this combination. |
| `STORAGE_TYPE` | `sqlite` | `sqlite` · `memory` (volatile). |
| `DATA_SOURCE_NAME` | `oxydraw.db` | SQLite path / `sqlite:` URL. |
| `CORS_ALLOWED_ORIGINS` | unset (same-origin only) | Comma-separated origins allowed to call the API cross-origin. Unset emits no CORS headers; `*` allows everything (development only). |
| `RUST_LOG` | `info` | Log filter (tracing env-filter syntax). |
| Auth vars | unset | `EXT_PASSWORD`, `PUBLIC_URL`, Google/GitHub OAuth credentials and overrides — see the [AUTH.md reference](AUTH.md#environment-variable-reference). Unset = scene library open. |

Request bodies are capped at 5 MB (matching the collaboration relay's scene-payload ceiling).

---

## Storage

SQLite is the default; the schema is created automatically on first start.

```ini
STORAGE_TYPE=sqlite
DATA_SOURCE_NAME=/var/lib/oxydraw/oxydraw.db
```

Keep the path inside `/var/lib/oxydraw` so it lives in the systemd `StateDirectory` and is
writable by the sandboxed service. [Back up](#backups) that file.

---

## Reverse proxy + TLS

Run oxydraw on `127.0.0.1:3002` and terminate TLS in front of it. The proxy **must**
forward WebSocket upgrades (Socket.IO). The UI talks to the backend same-origin, so no
host configuration is needed — just preserve the `Host` header as usual.

### Caddy

Handles TLS (Let's Encrypt) and WebSockets automatically:

```caddyfile
draw.example.com {
    reverse_proxy 127.0.0.1:3002
}
```

### nginx

```nginx
map $http_upgrade $connection_upgrade {
    default upgrade;
    ''      close;
}

server {
    listen 443 ssl http2;
    server_name draw.example.com;

    ssl_certificate     /etc/letsencrypt/live/draw.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/draw.example.com/privkey.pem;

    client_max_body_size 5m;          # match the 5 MB scene-payload cap

    location / {
        proxy_pass http://127.0.0.1:3002;
        proxy_http_version 1.1;
        proxy_set_header Host              $host;
        proxy_set_header X-Forwarded-For   $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header Upgrade           $http_upgrade;   # WebSocket
        proxy_set_header Connection        $connection_upgrade;
        proxy_read_timeout 3600s;                            # long-lived collab sockets
    }
}
```

The UI talks to the backend same-origin, so no extra backend host configuration is needed —
just forward the `Host` header and WebSocket upgrades as above, then restart.

---

## Upgrades

**APT repository:** `sudo apt update && sudo apt upgrade`. Your
`/etc/oxydraw/oxydraw.env` is preserved (it's a dpkg conf-file) and the unit is
restarted automatically.

**Manually built `.deb`:** rebuild the package from the new version (frontend first) and install
the file — same conf-file preservation and auto-restart:

```bash
sudo apt install ./oxydraw_<new-version>_<arch>.deb
```

**Other installs:** replace the binary and `sudo systemctl restart oxydraw`.

Schema changes are applied idempotently on start (`CREATE TABLE IF NOT EXISTS`), so no manual
migration step is required today. (Note: there is no down-migration / versioned-migration
machinery yet — back up before a major upgrade.)

## Backups

- **SQLite:** back up `/var/lib/oxydraw/oxydraw.db` (stop the service or use
  `sqlite3 … ".backup"` for a consistent copy). This holds the share-link documents **and
  scene image files** (`/api/files/*`), which are stored durably so they survive restarts.
- **Note:** *live-collaboration* scene snapshots (`/api/rooms/{id}/scene`) are **in-memory
  and ephemeral** — not persisted, not part of backups. Live clients re-save on change, so a
  restart only loses snapshots for rooms with no connected clients.

## Uninstall

```bash
sudo apt remove oxydraw            # keep config + data
sudo apt purge  oxydraw            # also remove /etc/oxydraw
sudo rm -rf /var/lib/oxydraw       # remove state (the SQLite DB) — irreversible
```

## Troubleshooting

| Symptom | Check |
| --- | --- |
| `systemctl status` shows failed | `journalctl -u oxydraw -e`. Common: bad `DATA_SOURCE_NAME`, unwritable DB path, port in use. |
| Collaboration doesn't sync | Reverse proxy not forwarding WebSocket upgrades (`Upgrade`/`Connection` headers), or not preserving the `Host` header. |
| UI is a blank placeholder page | The binary was built without embedding the frontend — rebuild after `make frontend`. |
| Uploads fail on large scenes | Payload over the 5 MB cap, or proxy `client_max_body_size` too low. |
