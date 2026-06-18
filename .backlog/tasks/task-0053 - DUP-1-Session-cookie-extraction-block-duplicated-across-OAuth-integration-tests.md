---
id: TASK-0053
title: >-
  DUP-1: Session-cookie extraction block duplicated across OAuth integration
  tests
status: Done
assignee:
  - TASK-0061
created_date: '2026-06-10 21:10'
updated_date: '2026-06-11 16:03'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
ordinal: 53000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/server/tests/ext_oauth_github.rs:103`

**What**: The get_all("set-cookie") / filter_map(to_str) / find(starts_with("ext_session=")) / split(';') extraction is duplicated at ext_oauth_github.rs:103-112 (inside sign_in) and ext_oauth_google.rs:150-158, and is structurally the same code as common/mod.rs:51-62 state_cookie (which does the identical scan for the ext_oauth_state cookie).

**Why it matters**: Three sites parse Set-Cookie headers with the same hand-rolled scan; a parameterized cookie(response, name) helper in tests/common would collapse all three and prevent assertion drift (the github copy additionally asserts HttpOnly, the google copy silently doesn't).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 tests/common/mod.rs exposes a cookie(r, name) -> String helper (state_cookie becomes cookie(r, "ext_oauth_state"))
- [ ] #2 ext_oauth_github.rs and ext_oauth_google.rs extract the ext_session cookie via the shared helper
- [ ] #3 Decide deliberately whether the HttpOnly assertion belongs in both flows and apply it consistently
<!-- AC:END -->
