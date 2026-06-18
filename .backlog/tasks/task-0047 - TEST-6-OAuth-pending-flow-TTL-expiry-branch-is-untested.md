---
id: TASK-0047
title: 'TEST-6: OAuth pending-flow TTL expiry branch is untested'
status: Done
assignee:
  - TASK-0060
created_date: '2026-06-10 21:09'
updated_date: '2026-06-11 15:53'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: low
ordinal: 47000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/server/src/oauth/mod.rs:254`

**What**: `PendingFlows::take` returns None for flows older than FLOW_TTL (line 257), but the unit tests only cover cap eviction and single-use. The test module's `pending_aged` helper already constructs flows with arbitrary ages, yet no test inserts a flow aged past FLOW_TTL and asserts `take` rejects it.

**Why it matters**: The TTL is a security control on the OAuth state lifetime; the expiry comparison (`<` vs `<=`, or a sign error in duration_since) could regress without any test failing, and the fix is a three-line test using existing infrastructure.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A test inserts a Pending aged >= FLOW_TTL and asserts flows.take(state) returns None
- [ ] #2 A boundary companion asserts a flow just inside the TTL is still taken
<!-- AC:END -->
