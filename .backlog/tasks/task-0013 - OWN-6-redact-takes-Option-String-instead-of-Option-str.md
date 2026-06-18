---
id: TASK-0013
title: 'OWN-6: redact takes &Option<String> instead of Option<&str>'
status: Done
assignee:
  - TASK-0028
created_date: '2026-06-07 11:55'
updated_date: '2026-06-07 13:13'
labels:
  - code-review-rust
  - ownership
dependencies: []
priority: low
ordinal: 13000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config.rs:71`

**What**: The helper `fn redact(secret: &Option<String>)` accepts a reference to an `Option`; it only ever calls `.as_ref().map(...)`, so `Option<&str>` is the idiomatic signature.

**Why it matters**: `&Option<T>` forces callers to hold an owned `Option` and is less flexible than `Option<&T>`; purely stylistic, single internal caller.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Signature changed to fn redact(secret: Option<&str>) -> ... (callers pass .as_deref())
- [x] #2 Debug impl call sites compile and still redact secrets
<!-- AC:END -->
