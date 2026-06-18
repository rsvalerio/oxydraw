---
id: TASK-0101
title: 'SEC-15: unaudited usize-to-u64 cast on untrusted body length'
status: Done
assignee: []
created_date: '2026-06-14 15:49'
updated_date: '2026-06-14 19:22'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
ordinal: 101000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `backend/crates/server/src/files.rs:87`, `backend/crates/server/src/routes.rs:51`

**What**: Both quota gates cast `body.len()` (usize) to `u64` with `as`. Lossless on 64-bit targets, but it is an unaudited `as` cast on an externally-controlled size value flowing into quota arithmetic — exactly the SEC-15 pattern.

**Why it matters**: Unaudited integer casts on untrusted sizes are a foot-gun if the type ever changes; `try_from`/explicit width documentation removes the risk.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Replace the cast with u64::try_from(body.len()) (handling the error) or annotate as provably-lossless on supported targets
- [x] #2 Apply the same treatment consistently in both files.rs and routes.rs
<!-- AC:END -->
