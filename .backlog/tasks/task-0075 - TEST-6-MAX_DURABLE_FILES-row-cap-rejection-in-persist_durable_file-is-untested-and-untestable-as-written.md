---
id: TASK-0075
title: >-
  TEST-6: MAX_DURABLE_FILES row-cap rejection in persist_durable_file is
  untested and untestable as written
status: Done
assignee:
  - TASK-0092
created_date: '2026-06-12 08:32'
updated_date: '2026-06-12 20:08'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: low
ordinal: 75000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/server/src/firebase.rs:299` (`MAX_DURABLE_FILES`), `crates/server/src/firebase.rs:372-382` (the cap check)

**What**: `persist_durable_file` rejects durable uploads with 507 once `count_files()` reaches `MAX_DURABLE_FILES` (100,000) — the SEC-33 row-cap added in task-0030. The branch has no test: `tests/storage.rs` covers the byte quota (`durable_uploads_past_the_file_quota_are_rejected`) and the name-length bound, but exercising the row cap would require inserting 100k rows because the constant is hardcoded with no injection point (unlike `max_files_bytes`, which is a `Config` field and therefore testable). This is the same untested-and-untestable shape as the LoginThrottle finding (task-0048), which was fixed by making the boundary injectable.

**Why it matters**: A regression that inverts the comparison or drops the check entirely would pass the full suite, silently removing one of the two SEC-33 bounds on the unauthenticated durable-upload path. Making the cap a `Config` field (defaulting to 100,000, like `max_files_bytes`) — or threading it as a parameter — lets an integration test pin the 507 at a small cap.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The durable-file row cap is injectable (config field or parameter) with the current 100,000 as default
- [x] #2 A test pins that an upload at the cap is rejected with 507 and one under the cap succeeds
<!-- AC:END -->
