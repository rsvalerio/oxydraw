---
id: TASK-0099
title: 'ERR-5: expect() in production session-mint entropy path'
status: Done
assignee:
  - TASK-0119
created_date: '2026-06-14 15:49'
updated_date: '2026-06-14 21:37'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
ordinal: 99000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `backend/crates/server/src/session.rs:37`

**What**: `OsRng.try_fill_bytes(...).expect("OS RNG must provide session-token entropy")` panics the request task if the OS CSPRNG ever fails. The `try_*` API exists precisely because OS entropy can fail.

**Why it matters**: A panic on a security-critical, request-reachable path is an uncontrolled crash rather than a controlled 500/retry. (Borderline per ERR-5 severity nuance: near-infallible, hence low.)
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 mint propagates the RNG failure as an error (StoreError/anyhow) instead of panicking
- [ ] #2 A test or documented rationale confirms the failure path returns an error, not a panic
<!-- AC:END -->
