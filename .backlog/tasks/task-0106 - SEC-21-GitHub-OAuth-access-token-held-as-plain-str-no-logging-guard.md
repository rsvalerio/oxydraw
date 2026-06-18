---
id: TASK-0106
title: 'SEC-21: GitHub OAuth access token held as plain str, no logging guard'
status: Done
assignee:
  - TASK-0119
created_date: '2026-06-14 15:50'
updated_date: '2026-06-14 21:36'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
ordinal: 106000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `backend/crates/server/src/oauth/github.rs:105,130-146`

**What**: `access_token = token.access_token().secret()` is held as a plain `&str` and passed to `bearer_auth(access_token)` in `api_get`. No current code logs it, but it is not wrapped in a redacting secret type, so a future `error!`/`warn!` interpolation could leak a live provider token. (Preventive hardening.)

**Why it matters**: Bearer access tokens are credentials; an accidental log of `access_token` would leak a live provider token (SEC-21 information disclosure).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Bind the access token as a SecretString/redacting wrapper so it is never directly formattable, or add a guard/comment preventing logging
- [ ] #2 Confirm via grep that no log statement in github.rs interpolates the raw access_token
<!-- AC:END -->
