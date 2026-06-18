---
id: TASK-0070
title: >-
  TEST-11: relay.rs negative-delivery proofs pass vacuously when the poll
  transport errors
status: Done
assignee:
  - TASK-0089
created_date: '2026-06-12 07:05'
updated_date: '2026-06-12 15:27'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: low
ordinal: 70000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/collab/tests/relay.rs:89-101` (asserted at `relay.rs:229-235` and `relay.rs:291-296`)

**What**: `PollingClient::wait_for` returns `None` for two indistinguishable reasons: the awaited packet never arrived within the bounded polls (the condition the negative proofs want), and `self.http.get(...)` failing outright (`let Ok(resp) = ... else { return None; }` at relay.rs:91). The two TEST-21 drain assertions — `assert!(member_a.wait_for(|p| p.contains("client-broadcast")).await.is_none())` in `server_broadcast_from_non_member_is_dropped` and `server_broadcast_relays_binary_payload_to_peers_only` — therefore also pass if the server panicked/died after the sentinel exchange or the session was torn down, i.e. exactly when delivery guarantees can no longer be observed.

**Why it matters**: These drains exist specifically to close a false-pass window (the comments say so), but they have a false-pass window of their own: a transport-level failure is silently read as "frame correctly dropped". A regression that crashes the relay mid-broadcast would still show green on the leak/echo assertions.

**Suggested shape**: make `wait_for` distinguish the outcomes (e.g. return `Result<Option<Vec<String>>, reqwest::Error>` or an enum `Matched(packets) / NoMatch / TransportError(e)`), and have the negative proofs assert `NoMatch` specifically — a transport error should fail the test.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Negative-delivery assertions in relay.rs fail (not pass) when the HTTP poll errors
- [x] #2 Positive waits keep their current behavior; all collab integration tests pass
<!-- AC:END -->
