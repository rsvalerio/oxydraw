---
id: TASK-0046
title: >-
  TEST-21: Sentinel-based non-delivery proofs in relay tests assume cross-socket
  handler ordering
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
ordinal: 46000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
<!-- scan confidence: candidates to inspect -->

**File**: `crates/collab/tests/relay.rs:206`

**What**: `server_broadcast_from_non_member_is_dropped` (line 206) and the echo check in `server_broadcast_relays_binary_payload_to_peers_only` (line 254) prove non-delivery by asserting the receiver's *first* client-broadcast is a later sentinel. The comments claim the offending frame was "handled first" because its HTTP POST completed first, but socketioxide spawns event handlers as tasks, so a 200 on the polling POST does not strictly guarantee the handler for socket X ran before the handler for socket Y.

**Why it matters**: If handler scheduling ever inverts, a regression (relayed outsider frame or sender echo) would arrive after the sentinel and the test would pass falsely — a false-negative (silent coverage loss), not a flaky failure, on two security/contract-critical assertions.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 After asserting the sentinel, the receiver drains one additional bounded poll and asserts no further client-broadcast packet arrived (closing the false-pass window)
- [ ] #2 Alternatively, verify socketioxide processes polling-POST packets synchronously before responding and document that guarantee at the assertion site
<!-- AC:END -->
