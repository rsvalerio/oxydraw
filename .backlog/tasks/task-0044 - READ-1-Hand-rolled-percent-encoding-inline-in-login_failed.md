---
id: TASK-0044
title: 'READ-1: Hand-rolled percent-encoding inline in login_failed'
status: Done
assignee:
  - TASK-0057
created_date: '2026-06-10 21:09'
updated_date: '2026-06-11 15:19'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: low
ordinal: 44000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
<!-- scan confidence: candidates to inspect -->

**File**: `crates/server/src/ext_routes.rs:322`

**What**: login_failed() implements URL form-encoding by hand as a bytes().map(...) chain with a per-byte match allocating a String per byte (lines 323-332), mixed into a helper whose job is building a redirect. The url crate (with form_urlencoded) is already in the dependency tree via oauth2/openidconnect.

**Why it matters**: Inline encoding obscures the helper's intent and re-implements a well-known, easy-to-get-subtly-wrong primitive; a named helper or the existing url/form_urlencoded machinery states intent and removes the per-byte allocations. Inputs are currently server-controlled literals, so this is purely a clarity/maintenance concern.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract the encoding into a named function (e.g. percent_encode_query) or replace it with form_urlencoded/url from the existing dependency tree
- [ ] #2 login_failed reads as: encode message, redirect to /?ext_auth_error=<encoded>; output for existing message literals is unchanged
<!-- AC:END -->
