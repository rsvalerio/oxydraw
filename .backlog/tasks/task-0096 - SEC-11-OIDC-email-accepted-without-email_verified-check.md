---
id: TASK-0096
title: 'SEC-11: OIDC email accepted without email_verified check'
status: Done
assignee:
  - TASK-0119
created_date: '2026-06-14 15:49'
updated_date: '2026-06-14 21:35'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
ordinal: 96000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `backend/crates/server/src/oauth/google.rs:140`

**What**: `exchange` reads `claims.email()` directly into `RemoteUser.email` and never inspects the `email_verified` claim. That email is the sole input to the `EXT_ALLOWED_EMAILS` allowlist gate in `ext_routes/auth.rs` (~line 509).

**Why it matters**: An IdP/Workspace account holding an unverified address matching an allowlisted email could pass the access gate, defeating the allowlist. `email_verified` is the canonical OIDC control for this.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 In exchange, treat email as None (or reject) when claims.email_verified() is not Some(true) before building RemoteUser
- [ ] #2 Add a test asserting an ID token with email_verified=false does not yield a usable email / is rejected at the allowlist
<!-- AC:END -->
