---
id: TASK-0029
title: >-
  SEC-18: OAuth sign-in has no user allowlist — any Google/GitHub account gets
  full library access
status: Done
assignee:
  - TASK-0057
created_date: '2026-06-10 21:08'
updated_date: '2026-06-11 15:19'
labels:
  - code-review-rust
  - security
dependencies: []
priority: high
ordinal: 29000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/server/src/ext_routes.rs:226`

**What**: `establish_session()` unconditionally upserts a user for whatever identity the provider returns and enrolls it in DEFAULT_ORG as 'member' (ext_routes.rs:226-235); there is no allowed-emails/domains/users config anywhere (core/src/config.rs defines none). With GOOGLE_CLIENT_ID/SECRET set on an internet-facing deployment, any Google or GitHub account holder worldwide can sign in. OWASP A01 Broken Access Control (authentication without authorization).

**Why it matters**: All members share the single default org's scene library, and list_scenes returns each scene's AES key (SceneView.key), so any stranger with a Google account can read, decrypt, and create scenes — almost certainly contrary to the deployer's expectation that "sign in with Google" restricts access to them.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 An allowlist config (e.g. EXT_ALLOWED_EMAILS / allowed domains / first-user-only enrollment) gates establish_session for OAuth identities; non-matching identities are rejected with a login error and no user/membership row is created
- [ ] #2 docs/AUTH.md documents the restriction and a test pins that a non-allowlisted identity cannot obtain a session
<!-- AC:END -->
