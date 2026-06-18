---
id: TASK-0005
title: 'SEC-7: Google OIDC nonce verification fails open when pending nonce is absent'
status: Done
assignee:
  - TASK-0026
created_date: '2026-06-07 11:24'
updated_date: '2026-06-07 13:00'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
ordinal: 5000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/server/src/oauth/google.rs:125`

**What**: `let nonce = Nonce::new(pending.nonce.unwrap_or_default());` — if `pending.nonce` is ever `None`, verification proceeds with an empty-string nonce instead of rejecting the flow. Today `authorize_url` always stores `Some(nonce)`, so the branch is unreachable, but the code is fail-open rather than fail-closed on an authentication check.

**Why it matters**: OWASP A07 (Identification and Authentication Failures). If a future refactor (e.g. a provider variant without nonces, or pending-flow serialization) produces a `None` nonce, ID-token verification silently loses replay protection instead of erroring.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A missing pending nonce returns AuthError::verification instead of defaulting to an empty nonce
- [x] #2 A unit test covers the None-nonce path rejecting the flow
<!-- AC:END -->
