---
id: TASK-0117
title: 'TEST-6: Folder rename happy path and multi-folder root listing are unasserted'
status: Done
assignee:
  - TASK-0121
created_date: '2026-06-14 19:07'
updated_date: '2026-06-14 21:57'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: low
ordinal: 117000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `backend/crates/server/tests/folders.rs:268` (rename reached only via the 404 case)

**What**: `rename_folder` (PATCH /folders/{id} with {"name": ...}) is exercised only against a missing folder (asserting 404); there is no test that renames an existing folder and asserts the new name is persisted via a follow-up list/GET. Similarly, the multi-folder root listing (more than one top-level folder) is never asserted. The happy path of rename is uncovered.

**Why it matters**: A rename handler that returns 200 but silently drops the name change (or writes the wrong field) would pass the current suite. Mutating endpoints should assert resulting state, not just the status code.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A test renames an existing folder and asserts a subsequent list/GET returns the updated name
- [ ] #2 A test creates 2+ top-level folders and asserts the root listing returns all of them
<!-- AC:END -->
