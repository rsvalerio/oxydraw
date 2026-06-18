---
id: TASK-0039
title: >-
  CONC-7: Single global Mutex serializes all reads on the hottest
  unauthenticated endpoints (LRU maps)
status: Done
assignee:
  - TASK-0058
created_date: '2026-06-10 21:08'
updated_date: '2026-06-11 15:26'
labels:
  - code-review-rust
  - idioms
dependencies: []
priority: low
ordinal: 39000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
<!-- scan confidence: candidates to inspect -->

**File**: `crates/server/src/firebase.rs:123`

**What**: `FirestoreState` (firebase.rs:123) and `StorageState` (firebase.rs:245) are `Arc<Mutex<BoundedMap>>` where even reads take the exclusive lock, because `BoundedMap::get` needs `&mut self` to stamp LRU recency (firebase.rs:94-101). batchGet and ?alt=media downloads — documented in-code as the hottest unauthenticated endpoints — therefore fully serialize on one mutex each. Mitigations are deliberate and documented (values behind Arc/Bytes keep critical sections to a refcount bump), so this is downgraded from its Medium baseline per the justified-violation guidance.

**Why it matters**: Under high concurrent collab load the exclusive lock on the read path becomes a throughput ceiling and a contention hotspot; an atomic-recency or sharded design would let reads proceed concurrently.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either profiling under realistic concurrency shows the mutex is not a bottleneck (documented), or recency stamping moves to an atomic (allowing RwLock/DashMap with shared reads) / the map is sharded
<!-- AC:END -->
