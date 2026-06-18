---
id: TASK-0097
title: 'SEC-19: Folder breadcrumb ancestor walk skips org isolation check'
status: Done
assignee:
  - TASK-0118
created_date: '2026-06-14 15:49'
updated_date: '2026-06-14 20:23'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
ordinal: 97000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `backend/crates/server/src/ext_routes/folders.rs:115-135`

**What**: `breadcrumb_to` resolves each ancestor via `find_folder(FolderId(&id))` without re-asserting `parent.org_id == folder.org_id`. Only the leaf folder passed in was org-checked by the caller.

**Why it matters**: A folder whose `parent_id` points (via data corruption or a future cross-org move bug) at another org's folder would surface that folder's name/id in the breadcrumb — a tenant-isolation leak (SEC-20). Defense in depth on the security crux of the new folder/ACL feature.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Assert/filter parent.org_id == folder.org_id in the walk, breaking the chain (like the NotFound arm) on mismatch
- [ ] #2 Add a test that a breadcrumb never includes a folder from a different org
<!-- AC:END -->
