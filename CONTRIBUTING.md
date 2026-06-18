# Contributing to oxydraw

Thanks for your interest in contributing! oxydraw is a self-hosted Excalidraw
collaboration backend written in Rust. Contributions of all kinds are welcome:
bug reports, fixes, docs, and features.

## Ground rule

> OxyDraw owns its frontend and its API contract.

The frontend is a custom Vite + React + TS SPA in `frontend/`, built on the
`@excalidraw/excalidraw` editor package. The Rust server exposes a small clean
same-origin API (`/api/v2`, `/api/files`, `/api/rooms`, Socket.IO) — no Firebase
emulation, no serve-time asset rewriting. Keep the editor pristine (drive it via
its props / imperative API, bump the package version) rather than forking it.

## Development setup

Requirements: Rust 1.85+ (see `rust-version` in `backend/Cargo.toml`). [bun](https://bun.sh)
(or Docker) is only needed to build the embedded frontend — `cargo build` and the full test
suite work without it. The Cargo workspace lives under `backend/`; the `make` targets below
run cargo there for you.

```bash
git clone https://github.com/rsvalerio/oxydraw
cd oxydraw
make run                       # → http://localhost:3002 (placeholder page without the frontend build)
make frontend                  # optional: build + embed the real UI (bun, or the oven/bun Docker image)
```

Configuration is environment-based — copy [`.env.example`](.env.example) to
`.env` and adjust as needed.

## Before you open a PR

All of these must pass (CI enforces them) — run them via `make` (which invokes
cargo in `backend/`), or `cd backend` and run the cargo commands directly:

```bash
make check                     # cargo fmt + clippy + test, all in backend/
```

If you touch dependencies, also run `make deny` (`cargo deny check` in `backend/`).

## Commit messages

This repo uses **[Conventional Commits](https://www.conventionalcommits.org/)** —
version bumps and the changelog are generated automatically from commit
messages by [cocogitto](https://github.com/cocogitto/cocogitto) (`cog bump --auto`
runs on green CI on `main`; see `cog.toml`).

Examples:

```
feat(collab): relay user-follow viewport events
fix(server): strip port from bracketed IPv6 Host headers
docs: clarify frontend build memory requirements
```

Use `feat:` for user-visible features (minor bump), `fix:` for bug fixes
(patch bump), and add `!` or a `BREAKING CHANGE:` footer for breaking changes.

## Pull requests

1. Fork and create a topic branch from `main`.
2. Keep PRs focused — one logical change per PR.
3. Add or update tests for behavior changes
   (`backend/crates/server/tests/`, `backend/crates/collab/tests/` — they're self-contained,
   no Docker needed).
4. Update docs (`README.md`, `docs/`) when behavior or configuration changes.

## Reporting issues

- **Bugs / features**: open a GitHub issue with reproduction steps or a clear
  use case.
- **Security vulnerabilities**: please do *not* open a public issue — see
  [SECURITY.md](SECURITY.md).

## Project layout

| Crate | Role |
| --- | --- |
| `backend/crates/core` | Domain types + storage trait + config. I/O-free. |
| `backend/crates/storage` | Storage backends: `sqlite` (default) and `memory`. |
| `backend/crates/collab` | Socket.IO collaboration relay (Excalidraw room protocol). |
| `backend/crates/server` | The `oxydraw` binary: axum router, share/file/scene APIs, frontend embedding. |
| `frontend/` | The frontend SPA (Vite + React + TS) on `@excalidraw/excalidraw`. |

See [AGENTS.md](AGENTS.md) for detailed project conventions (also consumed by
AI coding assistants).

## License

By contributing, you agree that your contributions will be licensed under the
[Apache License 2.0](LICENSE).
