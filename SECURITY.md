# Security Policy

## Supported versions

Only the latest release is supported with security updates.

## Reporting a vulnerability

Please **do not open a public issue** for security vulnerabilities.

Instead, report privately via
**[GitHub Security Advisories](https://github.com/rsvalerio/oxydraw/security/advisories/new)**
("Report a vulnerability" on the repo's Security tab).

Please include:

- A description of the issue and its impact
- Steps to reproduce (or a proof of concept)
- Affected version / commit

You can expect an acknowledgment within a few days. Once a fix is released,
the advisory will be published with credit to the reporter (unless you prefer
to remain anonymous).

## Scope notes

- Scene data in share links and live collaboration is **end-to-end encrypted
  by the Excalidraw client** — the server only stores and relays opaque blobs
  and never holds decryption keys. Issues with the E2E encryption itself
  belong upstream at
  [excalidraw/excalidraw](https://github.com/excalidraw/excalidraw).
- oxydraw is designed for self-hosting, typically behind a reverse proxy.
  Reports about deployments that ignore the hardening guidance in
  [`docs/DEPLOYMENT.md`](docs/DEPLOYMENT.md) may be considered out of scope,
  but err on the side of reporting.
