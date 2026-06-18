---
id: TASK-0004
title: >-
  DUP-3: Repeated 'error = &e as &(dyn std::error::Error)' cast pattern in
  tracing calls
status: Done
assignee:
  - TASK-0028
created_date: '2026-06-07 11:24'
updated_date: '2026-06-07 13:12'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
ordinal: 4000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/server/src/firebase.rs:338,347,367`, `crates/server/src/ext_routes.rs:195,361,456,470,539,613,631`, `crates/server/src/routes.rs:73,86`

**What**: Twelve `error!` call sites repeat the manual type ascription `error = &e as &(dyn std::error::Error + 'static)` (sometimes `&*e`) to record an error via tracing's Value impl for dyn Error.

**Why it matters**: If the pattern changes (e.g. switching to `tracing::field::display`/`%e`, or a tracing upgrade changing the dyn Error Value impl), twelve sites must be edited. A tiny helper fn or macro in one place removes the noise and the divergence risk.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A shared helper or macro encapsulates the dyn-Error cast for tracing fields
- [x] #2 All listed call sites use the helper; no remaining inline 'as &(dyn std::error::Error' casts in crates/server/src
<!-- AC:END -->
