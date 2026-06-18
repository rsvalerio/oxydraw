---
id: TASK-0041
title: 'FN-4: Provider::begin returns a 3-tuple with two adjacent same-typed Strings'
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
ordinal: 41000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/server/src/oauth/mod.rs:108`

**What**: Provider::begin (and the underlying GithubProvider::begin at oauth/github.rs:69 and OidcProvider::begin at oauth/google.rs:80) returns Result<(String, String, Pending), AuthError> where the two Strings are the authorization URL and the CSRF state. The meaning of each position is only conveyed by a doc comment ("returns (url, state, pending-flow record)").

**Why it matters**: Two adjacent String fields can be silently swapped at any call or implementation site with no compiler help; the consumer in ext_routes::auth_start destructures positionally, so a swap would redirect users to the CSRF state and store the URL as state.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace the tuple with a small named struct (e.g. BeginFlow { authorize_url: String, state: String, pending: Pending }) returned by Provider::begin and both provider impls
- [ ] #2 Call sites destructure by field name; behavior unchanged
<!-- AC:END -->
