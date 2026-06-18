---
id: TASK-0030
title: >-
  SEC-33: Durable file quota counts only payload bytes; unbounded object names
  and row count bypass it
status: Done
assignee:
  - TASK-0059
created_date: '2026-06-10 21:08'
updated_date: '2026-06-11 15:44'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
ordinal: 30000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/server/src/firebase.rs:342`

**What**: persist_durable_file enforces max_files_bytes against files_total_bytes(), which both backends compute as SUM(LENGTH(data)) only (sqlite.rs:311-317). The unauthenticated upload endpoint (handle_storage_upload, firebase.rs:286) accepts an attacker-chosen object name of unbounded length (query param, bounded only by hyper's request-head limit) and there is no cap on the number of files rows. OWASP A04 Insecure Design (unbounded resource consumption).

**Why it matters**: An unauthenticated client can loop tiny-payload uploads under files/shareLinks/ with long unique names: path bytes and row overhead are never charged against the quota, so the SQLite database grows without bound and fills the disk despite max_files_bytes. The in-memory StorageState similarly never charges key bytes (8192 entries x long names).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Object names are length-validated (e.g. <= 512 bytes) and rejected with 400 when oversized, on both upload and the in-memory path
- [ ] #2 Quota accounting charges path bytes (or a per-row overhead constant) and/or a max-row cap is enforced for the files table; a test pins that tiny-payload/long-name uploads cannot exceed the configured ceiling
<!-- AC:END -->
