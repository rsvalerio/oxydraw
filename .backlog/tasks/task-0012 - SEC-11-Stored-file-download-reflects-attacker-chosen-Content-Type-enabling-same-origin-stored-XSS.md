---
id: TASK-0012
title: >-
  SEC-11: Stored-file download reflects attacker-chosen Content-Type, enabling
  same-origin stored XSS
status: Done
assignee:
  - TASK-0024
created_date: '2026-06-07 11:55'
updated_date: '2026-06-07 12:42'
labels:
  - code-review-rust
  - security
dependencies: []
priority: high
ordinal: 12000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/server/src/firebase.rs:374`

**What**: `handle_storage_download` returns the stored `content_type` verbatim, and `handle_storage_upload` (via `parse_multipart_related`) takes that content type directly from the unauthenticated uploader's multipart part with no validation or allow-list. An attacker can upload an HTML/JS payload with `Content-Type: text/html` to an object path, then lure a victim to the same-origin `GET /v0/b/{bucket}/o/{object}` URL to run script in the app's origin.

**Why it matters**: Session cookies live on that origin. The global `X-Content-Type-Options: nosniff` does not prevent execution when the response itself declares `text/html`, and there is no `Content-Disposition: attachment` or CSP — this is same-origin stored XSS.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Raw object downloads are served with Content-Disposition: attachment and/or restricted to an inert content-type allow-list (e.g. image/*, application/octet-stream) so uploader-controlled HTML cannot execute on-origin
- [x] #2 A test uploads an object declaring Content-Type: text/html and asserts the download is not served as an active on-origin HTML document
<!-- AC:END -->
