---
id: TASK-0054
title: 'DUP-1: Identical redirect_uri-refusal guard in auth_start and auth_callback'
status: Done
assignee:
  - TASK-0057
created_date: '2026-06-10 21:10'
updated_date: '2026-06-11 15:19'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
ordinal: 54000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/server/src/ext_routes.rs:344`

**What**: The 7-line guard `let Some(redirect) = redirect_uri(&state, &headers, &provider_name) else { warn!(...); return login_failed("sign-in is not configured for this host"); }` is duplicated verbatim at crates/server/src/ext_routes.rs:344-350 (auth_start) and crates/server/src/ext_routes.rs:447-453 (auth_callback).

**Why it matters**: This is a security gate (SEC-29 host-poisoning defense): the warn message and the user-facing error must stay in lockstep across both handlers, and a future policy change edited in only one place would make start and callback disagree about which hosts are acceptable.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A single helper (e.g. fn required_redirect_uri(state, headers, provider) -> Result<String, Response>) owns the lookup, the warn, and the login_failed response
- [ ] #2 Both auth_start and auth_callback call the helper; the literal warn/error strings exist exactly once
<!-- AC:END -->
