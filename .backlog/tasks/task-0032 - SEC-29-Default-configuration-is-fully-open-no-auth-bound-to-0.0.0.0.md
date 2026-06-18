---
id: TASK-0032
title: 'SEC-29: Default configuration is fully open: no auth, bound to 0.0.0.0'
status: Done
assignee:
  - TASK-0062
created_date: '2026-06-10 21:08'
updated_date: '2026-06-11 16:06'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
ordinal: 32000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config.rs:104`

**What**: The zero-config defaults combine listen=0.0.0.0:3002 (config.rs:104-106) with ext_password=None and no OAuth provider, leaving the scene library (and everything else) open to anyone who can reach the socket; server startup only emits an info!-level notice (server/src/lib.rs:160-162). OWASP A05 Security Misconfiguration (insecure default). Documented as intentional for trusted-LAN use in docs/AUTH.md, which lowers the baseline severity.

**Why it matters**: A deployer who runs the binary unmodified on a machine with a public interface unknowingly exposes an open, writable scene library and unauthenticated storage endpoints to the internet.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either the default bind moves to 127.0.0.1 (with 0.0.0.0 as explicit opt-in), or the open-mode notice is elevated to warn! when listening on a non-loopback address with no auth configured
- [ ] #2 DEPLOYMENT.md states the open-by-default + 0.0.0.0 combination explicitly
<!-- AC:END -->
