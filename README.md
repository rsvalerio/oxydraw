<a id="readme-top"></a>

<!-- PROJECT SHIELDS -->
[![CI][ci-shield]][ci-url]
[![Release][release-shield]][release-url]
[![Contributors][contributors-shield]][contributors-url]
[![Issues][issues-shield]][issues-url]
[![Apache-2.0 License][license-shield]][license-url]

<!-- PROJECT LOGO -->
<br />
<div align="center">
  <h3 align="center">oxydraw</h3>

  <p align="center">
    A self-hosted <a href="https://excalidraw.com">Excalidraw</a> backend, written in Rust.
    <br />
    One binary: unmodified upstream UI, SQLite persistence, share links, and live collaboration.
    <br />
    <br />
    <a href="docs/DEPLOYMENT.md"><strong>Deployment guide »</strong></a>
    <br />
    <br />
    <a href="https://github.com/rsvalerio/oxydraw/issues/new?labels=bug">Report Bug</a>
    &middot;
    <a href="https://github.com/rsvalerio/oxydraw/issues/new?labels=enhancement">Request Feature</a>
  </p>
</div>

<!-- TABLE OF CONTENTS -->
<details>
  <summary>Table of Contents</summary>
  <ol>
    <li><a href="#about-the-project">About The Project</a>
      <ul>
        <li><a href="#what-is-excalidraw-and-what-is-oxydraw">What is Excalidraw, and what is oxydraw?</a></li>
        <li><a href="#features">Features</a></li>
        <li><a href="#built-with">Built With</a></li>
        <li><a href="#workspace-layout">Workspace Layout</a></li>
      </ul>
    </li>
    <li><a href="#getting-started">Getting Started</a>
      <ul>
        <li><a href="#prerequisites">Prerequisites</a></li>
        <li><a href="#quick-start">Quick Start</a></li>
        <li><a href="#develop">Develop</a></li>
        <li><a href="#frontend">Frontend</a></li>
      </ul>
    </li>
    <li><a href="#how-it-works">How It Works</a></li>
    <li><a href="#deployment">Deployment</a></li>
    <li><a href="#release-process">Release Process</a></li>
    <li><a href="#roadmap">Roadmap</a></li>
    <li><a href="#contributing">Contributing</a></li>
    <li><a href="#license">License</a></li>
    <li><a href="#contact">Contact</a></li>
    <li><a href="#acknowledgments">Acknowledgments</a></li>
  </ol>
</details>

<!-- ABOUT THE PROJECT -->
## About The Project

oxydraw serves a **custom whiteboard UI built on the `@excalidraw/excalidraw` editor** from a
single Rust binary, with embedded SQLite persistence, anonymous share links, a real-time
collaboration relay, and a small clean same-origin API. No accounts, no external services —
`cargo run` and draw.

> **Design rule:** OxyDraw owns its frontend and its API contract. The UI is a custom
> Vite + React SPA in [`frontend/`](frontend/) on the `@excalidraw/excalidraw` npm package; the Rust
> server defines a clean same-origin API (`/api/v2`, `/api/files`, `/api/rooms`, Socket.IO).
> No Firebase emulation, no serve-time asset rewriting.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

### What is Excalidraw, and what is oxydraw?

The name "Excalidraw" covers several things; oxydraw is exactly one of them:

* **[Excalidraw, the editor](https://github.com/excalidraw/excalidraw)** — the free,
  open-source npm package: infinite canvas, hand-drawn style, dark mode, image support,
  shape libraries, localization, PNG/SVG/clipboard export, the open `.excalidraw` format,
  the full tool set (rectangle, ellipse, diamond, arrow, line, free-draw, eraser, …),
  arrow binding & labeled arrows, undo/redo, zoom and panning. All of this runs in the
  browser and needs no server.
* **[excalidraw.com](https://excalidraw.com)** — the official hosted app built on that
  editor. It adds PWA/offline support and local-first autosave (also client-side), plus
  the three features that *do* need a backend: shareable links, real-time collaboration,
  and end-to-end encryption of everything those two send to the server.
* **[Excalidraw+](https://plus.excalidraw.com)** — the Excalidraw team's commercial SaaS
  (cloud workspaces, teams, presentations, …). A separate paid product: oxydraw is not
  affiliated with it and does not replicate it.
* **oxydraw** (this project) — a self-hosted replacement for the *backend* behind
  excalidraw.com. It embeds the unmodified editor and reimplements the server side in
  Rust, so you get the excalidraw.com experience on your own machine.

### Features

Everything in the editor list above works as-is — oxydraw serves the pristine upstream UI.
On top of that, oxydraw provides:

* 🦀 **One static Rust binary** — UI embedded at build time; `cargo run` and draw.
* 🔗 **Shareable links** — scenes stored as opaque E2E-encrypted blobs; the server never
  sees their contents.
* 🤼 **Real-time collaboration** — Socket.IO relay implementing the Excalidraw room
  protocol, with end-to-end-encrypted scene + cursor sync and image file transfer.
* 🧩 **Clean same-origin API** — share links, scene image blobs (`/api/files`), and per-room
  scene snapshots (`/api/rooms`); no Firebase, no Google services.
* 🗃️ **SQLite persistence** — a single database file, no external services.
* 📚 **Scene library** — optional server-side saved-scene library (a panel in the app), gated
  by nothing, a shared password, or Sign in with Google / GitHub ([`docs/AUTH.md`](docs/AUTH.md)).
* 🚀 **Self-host friendly** — Docker image, `.deb` + systemd unit, prebuilt binaries
  ([`docs/DEPLOYMENT.md`](docs/DEPLOYMENT.md)).

<p align="right">(<a href="#readme-top">back to top</a>)</p>

### Built With

* [![Rust][rust-shield]][rust-url]
* [axum](https://github.com/tokio-rs/axum) — HTTP router
* [socketioxide](https://docs.rs/socketioxide) — Socket.IO collaboration relay
* [sqlx](https://github.com/launchbadge/sqlx) + SQLite — persistence
* [@excalidraw/excalidraw](https://www.npmjs.com/package/@excalidraw/excalidraw) — the editor component the `frontend/` SPA is built on

<p align="right">(<a href="#readme-top">back to top</a>)</p>

### Workspace Layout

The repo splits cleanly by toolchain: **`backend/`** holds the Rust Cargo workspace,
**`frontend/`** holds the Vite + React SPA. The frontend builds into
`backend/crates/server/assets/`, which the server embeds at compile time — so the
deliverable stays a single self-contained binary.

| Path | Role |
| --- | --- |
| `backend/crates/core` | Domain types + the storage trait (`DocumentStore`/`Store`) + config. I/O-free. |
| `backend/crates/storage` | Storage backends: `sqlite` (sqlx, default) and `memory` (dev/tests). |
| `backend/crates/collab` | socketioxide Socket.IO relay implementing the Excalidraw room protocol. |
| `backend/crates/server` | The `oxydraw` app (lib + bin): axum router, share/file/scene APIs, frontend embedding. |
| `frontend/` | The frontend SPA (Vite + React + TS) on `@excalidraw/excalidraw`: editor, share, collab, persistence. |

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- GETTING STARTED -->
## Getting Started

### Prerequisites

* Rust 1.85+ (`rustup`)
* For the frontend only: [bun](https://bun.sh) — or Docker (the build then runs in the `oven/bun` image, so no host JS toolchain is needed)

### Quick Start

```bash
make frontend                  # one-time: build + embed the UI (needs Docker, see below)
cargo run                      # → http://localhost:3002
```

Skipping the first step still works — the server just serves a placeholder page instead of
the UI.

### Develop

```bash
cargo run                      # serves on 0.0.0.0:3002 (STORAGE_TYPE=sqlite default)
cargo fmt --all
cargo clippy --all --all-features -- -D warnings
cargo test --all --all-features    # self-contained; no Docker needed
```

Configuration is via environment variables, loaded from `.env` — see
[`.env.example`](.env.example) for the annotated list and
[`docs/DEPLOYMENT.md`](docs/DEPLOYMENT.md#configuration-reference) for the full reference.

### Frontend

The web UI is a custom Vite + React + TypeScript SPA in [`frontend/`](frontend/) built on the
[`@excalidraw/excalidraw`](https://www.npmjs.com/package/@excalidraw/excalidraw) editor
package (currently **v0.18.x**). The SPA talks to the backend's same-origin API directly
(`/api/v2`, `/api/files`, `/api/rooms`, Socket.IO) — there is no Firebase emulation and no
serve-time asset rewriting. App-level behavior (share links, collaboration, persistence, file
sync) lives in `frontend/src/`.

Build it into the binary (rust-embed reads `backend/crates/server/assets/`):

```bash
make frontend                   # bun build in frontend/ (or the oven/bun Docker image if bun isn't installed)
make release                    # builds the frontend, then cargo build --release in backend/
```

`make frontend` uses host `bun` when present, otherwise the `oven/bun` Docker image — either way
no Node toolchain is required. The Docker image build does this automatically (frontend stage →
embed → compile), so `make docker` needs no host JS toolchain. Without a build, `assets/` holds
only a `.gitkeep` and the server serves a small placeholder page, so `cargo build` always works.
Built assets are never committed (gitignored).

To bump the editor: change the `@excalidraw/excalidraw` version in `frontend/package.json`, run
`bun install` in `frontend/`, and rebuild.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- HOW IT WORKS -->
## How It Works

The `frontend/` SPA keeps drawing, reconciliation, and local persistence client-side, and uses
four backend surfaces; the server only ever stores opaque, end-to-end-encrypted bytes (the
share/room AES key lives in the URL fragment and never reaches it):

- **Share links** — `POST /api/v2/post/` stores the raw E2E-encrypted scene blob in SQLite
  and returns `{"id"}`; `GET /api/v2/{id}` returns the bytes.
- **Live collaboration** — `backend/crates/collab` implements the Excalidraw room protocol over
  Socket.IO using [`socketioxide`](https://docs.rs/socketioxide) (Engine.IO v3 + v4). Events:
  `init-room`, `join-room`, `first-in-room`, `new-user`, `room-user-change`,
  `server-broadcast` → `client-broadcast`. Only relays opaque, E2E-encrypted frames. The
  `frontend/src/collab/` client encrypts scene + cursor updates and reconciles remote edits.
  (Known omission: `user-follow` viewport following is not relayed.)
- **Scene image files** — `PUT`/`GET /api/files/{id}` store opaque file blobs under their
  content-addressed id, durable in SQLite.
- **Collab scene snapshots** — `PUT`/`GET /api/rooms/{id}/scene` hold one opaque snapshot per
  room (in-memory, bounded) so a client joining an empty room can restore the last state;
  live rooms re-save on change.

Tests: `backend/crates/server/tests/{share,storage,rooms}.rs` round-trip each surface;
`backend/crates/collab/tests/` exercises the relay. **Full interop against a real browser session
(two-browser collab, including an image element) remains a manual gate.**

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- DEPLOYMENT -->
## Deployment

See **[`docs/DEPLOYMENT.md`](docs/DEPLOYMENT.md)** for the full operations guide: installing
on a VM via a `.deb` package, running under **systemd**, SQLite storage, a reverse-proxy +
TLS config, upgrades, and backups. Packaging inputs live in `packaging/` (systemd unit + env
file); the `.deb` is built with `cargo deb -p oxydraw` (build the frontend first so
the UI is embedded).

The scene library can be gated by a shared password (`EXT_PASSWORD`) or real
accounts via **Sign in with Google / GitHub** — see **[`docs/AUTH.md`](docs/AUTH.md)** for
the cloud-side setup (OAuth client registration, redirect URIs, env vars).

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- RELEASE PROCESS -->
## Release Process

- **CI** (`.github/workflows/ci.yml`): fmt / check / clippy / build / test / `cargo deny`.
- **Version bump** (`bump.yml`): `cog bump --auto` on green CI on `main` (conventional commits).
- **Binaries + Homebrew** (`release.yml` + `dist-workspace.toml`): cargo-dist on tags.
  Regenerate this workflow with `dist generate` after editing `dist-workspace.toml`.
- **Docker** (`docker.yml`): multi-arch image to GHCR — the primary self-hosting artifact.
- **Debian package**: `cargo deb -p oxydraw` produces a `.deb` (systemd unit + config);
  see [`docs/DEPLOYMENT.md`](docs/DEPLOYMENT.md).

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- ROADMAP -->
## Roadmap

- [ ] Relay `user-follow` viewport following in the collab protocol
- [ ] Automated interop test against a real Excalidraw JS client

See the [open issues](https://github.com/rsvalerio/oxydraw/issues) for a full list of
proposed features and known issues.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- CONTRIBUTING -->
## Contributing

Contributions welcome — see [CONTRIBUTING.md](CONTRIBUTING.md) for dev setup and the PR
checklist (conventional commits required; the changelog and version bumps are generated
from them). Security issues: please report privately per [SECURITY.md](SECURITY.md).

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- LICENSE -->
## License

Distributed under the Apache License 2.0. See [`LICENSE`](LICENSE) for more information.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- CONTACT -->
## Contact

Rodrigo Valeri — [@rsvalerio](https://github.com/rsvalerio)

Project Link: [https://github.com/rsvalerio/oxydraw](https://github.com/rsvalerio/oxydraw)

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- ACKNOWLEDGMENTS -->
## Acknowledgments

* [Excalidraw](https://github.com/excalidraw/excalidraw) — the whole point
* [Best-README-Template](https://github.com/othneildrew/Best-README-Template) — this README's structure
* The previous incarnation of this project (multi-canvas fork UI, OIDC accounts, per-user
  canvases, server-side libraries, PostgreSQL) is preserved at the `pre-simplify` git tag.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- MARKDOWN LINKS & IMAGES -->
[ci-shield]: https://img.shields.io/github/actions/workflow/status/rsvalerio/oxydraw/ci.yml?branch=main&style=for-the-badge&label=CI
[ci-url]: https://github.com/rsvalerio/oxydraw/actions/workflows/ci.yml
[release-shield]: https://img.shields.io/github/v/release/rsvalerio/oxydraw?style=for-the-badge
[release-url]: https://github.com/rsvalerio/oxydraw/releases
[contributors-shield]: https://img.shields.io/github/contributors/rsvalerio/oxydraw.svg?style=for-the-badge
[contributors-url]: https://github.com/rsvalerio/oxydraw/graphs/contributors
[issues-shield]: https://img.shields.io/github/issues/rsvalerio/oxydraw.svg?style=for-the-badge
[issues-url]: https://github.com/rsvalerio/oxydraw/issues
[license-shield]: https://img.shields.io/github/license/rsvalerio/oxydraw.svg?style=for-the-badge
[license-url]: https://github.com/rsvalerio/oxydraw/blob/main/LICENSE
[rust-shield]: https://img.shields.io/badge/Rust-1.85+-000000?style=for-the-badge&logo=rust&logoColor=white
[rust-url]: https://www.rust-lang.org/
