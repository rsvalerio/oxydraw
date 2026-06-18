---
id: TASK-0006
title: >-
  SEC-19: Anonymous document access in get_document is undocumented by-design
  behavior
status: Done
assignee:
  - TASK-0024
created_date: '2026-06-07 11:24'
updated_date: '2026-06-07 12:43'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
ordinal: 6000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/server/src/routes.rs:81`

**What**: `GET /api/v2/{id}` serves any stored document to any unauthenticated client that knows the UUID (capability-URL sharing, matching Excalidraw semantics). The handler carries no comment stating this is intentional, and no integration test pins the no-auth contract.

**Why it matters**: OWASP A01 (Broken Access Control) — not a vulnerability today, but undocumented intentional-IDOR is a regression trap: a future contributor may either assume per-user isolation exists, or "fix" the endpoint by adding auth and break share links.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 get_document carries a comment stating anonymous capability-URL access is intentional
- [x] #2 An integration test asserts a stored document is readable without any session/auth
<!-- AC:END -->
