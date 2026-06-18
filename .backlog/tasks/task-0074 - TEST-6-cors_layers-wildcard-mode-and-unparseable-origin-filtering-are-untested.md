---
id: TASK-0074
title: >-
  TEST-6: cors_layer's wildcard (*) mode and unparseable-origin filtering are
  untested
status: Done
assignee:
  - TASK-0092
created_date: '2026-06-12 08:32'
updated_date: '2026-06-12 20:06'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: low
ordinal: 74000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/server/src/lib.rs:78-104` (`cors_layer`); existing coverage in `crates/server/tests/security_headers.rs:37-83`

**What**: `cors_layer` has four behavioral branches: unset/empty (no CORS headers), literal `*` (allow-everything dev mode), a parseable allowlist, and per-entry filtering of unparseable `CORS_ALLOWED_ORIGINS` entries (warn + skip, lib.rs:90-96). Integration tests cover only the first and third. Untested: (a) `CORS_ALLOWED_ORIGINS=*` actually grants an arbitrary origin, and (b) a list containing a malformed entry (e.g. `"https://ok.example, bad\u{7f}value"`) still grants the valid origins and does not panic or grant the malformed one. Branch (b) silently shrinks the allowlist — a config typo locking out a legitimate frontend would only show as a warn line today.

**Why it matters**: CORS is a security perimeter; its config-parsing branches are exactly the code that only misbehaves in production (nobody passes malformed origins in dev). The two uncovered branches are cheap to pin in `security_headers.rs` alongside the existing allowlist tests.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A test pins that CORS_ALLOWED_ORIGINS=* reflects an arbitrary Origin (dev wildcard mode)
- [x] #2 A test pins that a list with an unparseable entry still grants the remaining valid origins and grants nothing for the malformed entry
<!-- AC:END -->
