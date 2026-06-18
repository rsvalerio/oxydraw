---
id: TASK-0048
title: >-
  TEST-6: LoginThrottle window-expiry recovery path is untested and untestable
  as written
status: Done
assignee:
  - TASK-0060
created_date: '2026-06-10 21:09'
updated_date: '2026-06-11 15:53'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: low
ordinal: 48000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/server/src/ext_routes.rs:71`

**What**: `LoginThrottle::allow` prunes failures older than LOGIN_FAILURE_WINDOW (60s) so legitimate users recover after an attack, and the integration test ext_auth.rs:129 even comments "(it expires with the window)" — but no test exercises the pruning branch, because `Instant::now()` is hardcoded and waiting 60s in a test is not viable.

**Why it matters**: The recovery path is what prevents the throttle from becoming a permanent denial-of-service against the legitimate user; a pruning regression (e.g. comparison flipped) would lock out logins forever and no test would notice.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 LoginThrottle accepts an injectable clock (or its allow/record methods take now: Instant) so a unit test can advance time past LOGIN_FAILURE_WINDOW
- [ ] #2 A unit test fills the failure budget, advances the injected clock past the window, and asserts allow() returns true again
<!-- AC:END -->
