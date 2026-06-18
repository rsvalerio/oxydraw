---
id: TASK-0007
title: 'SEC-29: No startup warning when OAuth is enabled without PUBLIC_URL'
status: Done
assignee:
  - TASK-0026
created_date: '2026-06-07 11:24'
updated_date: '2026-06-07 13:01'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
ordinal: 7000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/server/src/ext_routes.rs:286`

**What**: When `PUBLIC_URL` is unset, the OAuth redirect_uri is derived from the request Host header, gated to loopback/localhost hosts. docs/AUTH.md requires admins to set PUBLIC_URL in production, but the server emits no startup warning when an OAuth provider is configured without it.

**Why it matters**: OWASP A05 (Security Misconfiguration). A production deployment behind a misconfigured reverse proxy that forwards a loopback Host header could derive attacker-influenced redirect URIs; a loud startup warning makes the misconfiguration visible before it bites.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Server logs a warning at startup when any OAuth provider is enabled and PUBLIC_URL is not set
- [x] #2 Warning text points at docs/AUTH.md guidance for production deployments
<!-- AC:END -->
