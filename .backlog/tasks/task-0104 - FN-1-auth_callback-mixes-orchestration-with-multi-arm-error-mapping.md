---
id: TASK-0104
title: 'FN-1: auth_callback mixes orchestration with multi-arm error mapping'
status: Done
assignee:
  - TASK-0119
created_date: '2026-06-14 15:50'
updated_date: '2026-06-14 21:37'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: low
ordinal: 104000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `backend/crates/server/src/ext_routes/auth.rs:469-528`

**What**: `auth_callback` is ~60 lines spanning validation, redirect derivation, exchange, three exchange-error arms, allowlist gating, and session establishment — above the 50-line single-abstraction-level guideline.

**Why it matters**: Long handlers concentrate security-critical branching, making it harder to verify every error path fails closed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract the exchange(...) match (the three AuthError arms -> login_failed) into a named helper returning Result<RemoteUser, Response>
- [ ] #2 Keep the post-extraction handler under ~40 lines with one abstraction level
<!-- AC:END -->
