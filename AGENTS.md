# AGENTS.md

Instructions for AI coding agents working on `oxydraw` — a self-hosted, Excalidraw-based
whiteboard in Rust. See [README.md](README.md) for the product overview, quick start, and
frontend build instructions.

**Design rule: OxyDraw owns its frontend and its API contract.** The frontend is a custom
Vite + React + TypeScript SPA in [`frontend/`](frontend/) built on the `@excalidraw/excalidraw` npm
package (the editor *component* — not the excalidraw.com app). The Rust server defines a
small, clean, same-origin API the SPA talks to directly (`/api/v2` shares, `/api/files`,
`/api/rooms/{id}/scene`, the Socket.IO relay); there is **no Firebase emulation and no
serve-time asset rewriting** — assets are served verbatim. Keep the *editor* pristine: drive
it through its public props / imperative API and version bump, rather than forking the
package. App-level behavior (collab, share, persistence, file sync) lives in `frontend/src/`.

## Core workflow

- Don't assume. Don't hide confusion. Surface tradeoffs.
- Minimum code that solves the problem. Nothing speculative.
- Touch only what you must. Clean up only your own mess.
- Define success criteria. Loop until verified.
- Prefer existing project patterns over new abstractions.
- Put tests next to the code they cover with `#[cfg(test)] mod tests` when practical;
  cross-crate / network tests go in `backend/crates/<crate>/tests/`.
- Add or update tests for new behavior.
- Rust edition is 2021, MSRV 1.85 (async closures). Treat clippy warnings as errors.

## Rust implementation guardrails

For any non-trivial Rust change, read the `code-review-rust` skill *before* editing and
follow its rules as acceptance criteria. Do not file backlog tasks during implementation.

Before declaring a change done, run (cargo lives in `backend/`; `make check` does this for you):

```
cd backend && cargo fmt --all
cd backend && cargo clippy --all --all-features -- -D warnings
cd backend && cargo test --all --all-features
```

The test suite is self-contained (no Docker daemon needed).

## Code map

Top-level split by toolchain: `backend/` is the Rust Cargo workspace (run `cargo` there, or
use the `make` targets); `frontend/` is the SPA. The frontend builds into
`backend/crates/server/assets/`, embedded into the binary at compile time.

- `backend/crates/core/`    — domain types (`Document`), the storage trait (`DocumentStore` /
  `Store`), and env config. No I/O backends here.
- `backend/crates/storage/` — trait implementations: `sqlite` (sqlx, feature-gated, default) and
  `memory` (dev/tests). Both are held to one shared `test_support` contract (one test per store concern).
  `select_store` is async; `sqlite` is the default `STORAGE_TYPE`.
- `backend/crates/collab/`  — socketioxide Socket.IO relay: the Excalidraw room protocol
  (`init-room` / `join-room` / `first-in-room` / `new-user` / `room-user-change` /
  `server-broadcast` → `client-broadcast`). Room membership is tracked in-crate.
- `backend/crates/server/`  — the app. `lib.rs` (`build_router`, `run`, `AppState`) so tests can
  drive the HTTP surface; thin `main.rs`. `routes.rs`: anonymous shares (`/api/v2/post/`,
  `/api/v2/{id}`) and the router assembly. `files.rs`: the clean file-blob API
  (`PUT`/`GET /api/files/{id}`) for scene images, durable via `Store`. `rooms.rs`: per-room
  collab scene snapshots (`/api/rooms/{id}/scene`), in-memory and bounded. `bounded_map.rs`:
  the LRU cap backing those snapshots. `frontend.rs`: `rust-embed` static + SPA-fallback
  serving from `backend/crates/server/assets/`; placeholder when unbuilt. `ext_routes.rs`: the
  scene-library API (`/api/ext/*` — save/list scenes + auth), surfaced natively in the SPA
  (`frontend/src/library/`). `oauth/` + `session.rs`: optional sign-in gating that library — shared
  password (`EXT_PASSWORD`), Google (OIDC), GitHub (OAuth2), SQLite-persisted sessions; see
  `docs/AUTH.md`. Mounts the collab layer.
- `frontend/`            — the frontend SPA (Vite + React + TS) on `@excalidraw/excalidraw`.
  `src/App.tsx` renders the editor; `src/share.ts` + `src/crypto.ts` the share-link flow;
  `src/localData.ts` localStorage persistence; `src/files.ts` file transfer;
  `src/collab/` the collaboration client (`Collab.ts` + the wire `protocol.ts`);
  `src/library/` the scene-library UI (save/open + sign-in panel) over `/api/ext/*`.

## Frontend build

`make frontend` builds the SPA with **bun** — host `bun` if it's on PATH, otherwise inside the
`oven/bun` Docker image (no Node toolchain needed either way) — and vite writes the bundle straight
into `backend/crates/server/assets/` for rust-embed. Built assets are gitignored; without them the
server serves a placeholder. `make docker` runs the equivalent in the Dockerfile's `frontend` stage
(also `oven/bun`), so the image build needs no host JS toolchain. The editor's hand-drawn fonts are
copied into the build (vite-plugin-static-copy) and served same-origin — no CDN. Dependencies are
locked in `frontend/bun.lock` (committed); the build uses `bun install --frozen-lockfile`.

To bump the editor: change the `@excalidraw/excalidraw` version in `frontend/package.json`,
`bun install`, and re-test (especially the collab reconcile path and share round-trip, which
depend on the package's element/reconcile types).

## Architecture notes

- Server-side surface: share-link POST/GET (raw encrypted bytes ↔ `{"id"}` JSON), file
  blobs (`/api/files/{id}`), per-room scene snapshots (`/api/rooms/{id}/scene`), the
  Socket.IO room relay, and the optionally auth-gated scene library (`/api/ext/*`). The
  server only ever stores opaque, end-to-end-encrypted bytes; the room/share AES key lives in
  the URL fragment and never reaches it. Drawing, reconciliation, and local persistence are
  client-side.
- Scene snapshots are in-memory and ephemeral (bounded): live clients re-save on change, so a
  restart only loses snapshots for rooms with no connected clients. File blobs are durable.
- The collab client speaks the **unchanged** `backend/crates/collab` relay protocol; the relay is
  protocol-agnostic (opaque frames + membership), so changes there are rare.
- Known omission: top-level `user-follow` events aren't relayed, so viewport follow-mode
  no-ops.
- Remaining manual gate: a real two-browser collab session (including an image element) in a
  browser — the endpoint shapes and the relay handshake are tested, but the editor's
  interactive rendering is not exercised headlessly.
- Out of scope (not planned): per-user canvases (`/api/v2/kv`), PostgreSQL, S3, OpenAI
  proxy. The pre-rewrite implementation is preserved at the `pre-simplify` tag. Do not
  reintroduce unless explicitly asked.
