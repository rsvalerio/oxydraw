---
id: TASK-0116
title: 'TEST-6: Folder move-cycle rejection test covers only the depth-2 case'
status: Done
assignee:
  - TASK-0121
created_date: '2026-06-14 19:06'
updated_date: '2026-06-14 21:57'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: low
ordinal: 116000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `backend/crates/server/tests/folders.rs:224` (`moving_a_folder_into_its_own_descendant_is_rejected`)

**What**: The cycle-rejection test only exercises a depth-2 cycle (move A under its immediate child B). It does not cover the multi-level descendant case (A->B->C, then move A under C) nor the self-parent case (move A under A). The backing ancestor-walk is exactly the kind of multi-level traversal where an off-by-one (stopping one level short) passes a depth-2 test but lets a depth-3 cycle through.

**Why it matters**: A folder cycle that escapes detection corrupts the tree invariant and can cause infinite loops in breadcrumb/descendant traversal at runtime. The single shallow case under-tests the branch.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A test reparents a grandparent under a depth-3 descendant (A->B->C, move A under C) and asserts 409
- [ ] #2 A test asserts moving a folder under itself (parent_id = its own id) returns 409 or the documented error status
<!-- AC:END -->
