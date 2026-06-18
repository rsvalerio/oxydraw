---
id: TASK-0077
title: >-
  DUP-1: ext_auth.rs hand-rolls session-cookie extraction that
  tests/common::cookie already provides
status: Done
assignee:
  - TASK-0092
created_date: '2026-06-12 08:59'
updated_date: '2026-06-12 20:09'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
ordinal: 77000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
<!-- scan confidence: candidates to inspect -->

**File**: `crates/server/tests/ext_auth.rs:92`, `crates/server/tests/ext_auth.rs:193-194`, `crates/server/tests/ext_auth.rs:236-237`

**What**: Three sites in `ext_auth.rs` extract the `ext_session=...` pair from a response via the manual `r.headers().get("set-cookie") ... .split(';').next()` sequence, duplicating `tests/common/mod.rs::cookie` (mod.rs:74-86), which the OAuth test binaries already use — the consolidation done for TASK-0053 (commit d778ebd) covered `ext_oauth_github.rs`/`ext_oauth_google.rs` but not this binary. Note the *attribute-asserting* sites (ext_auth.rs:80-89, 156-161, 177-183) legitimately read the raw `Set-Cookie` header to check `HttpOnly`/`Secure` and are not candidates; only the three pair-extraction sites are.

**Why it matters**: Same rationale as TASK-0053 — the helper encodes the multi-`Set-Cookie` lookup correctly (uses `get_all`, panics with a named message when absent), while the hand-rolled sites use single-header `get` + bare `unwrap`, which would mislocate a failure if a second cookie is ever set on these responses (as the OAuth sign-in path already does). Finishing the consolidation keeps one canonical extraction.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The three pair-extraction sites in ext_auth.rs use common::cookie instead of manual set-cookie parsing
- [x] #2 Attribute-asserting sites (HttpOnly/Secure checks) are left reading the raw header
<!-- AC:END -->
