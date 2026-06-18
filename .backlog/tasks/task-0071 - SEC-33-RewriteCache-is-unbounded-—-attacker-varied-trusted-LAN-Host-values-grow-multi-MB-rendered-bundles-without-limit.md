---
id: TASK-0071
title: >-
  SEC-33: RewriteCache is unbounded — attacker-varied trusted-LAN Host values
  grow multi-MB rendered bundles without limit
status: Done
assignee:
  - TASK-0090
created_date: '2026-06-12 08:31'
updated_date: '2026-06-12 17:42'
labels:
  - code-review-rust
  - security
dependencies: []
priority: high
ordinal: 71000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/server/src/frontend.rs:31` (struct), `crates/server/src/frontend.rs:59-91` (population), `crates/server/src/frontend.rs:126-150` (trust gate)

**What**: `RewriteCache` is a plain `Arc<Mutex<HashMap<String, Bytes>>>` with no entry or byte bound. The cache key is `(effective host, asset name)`, and when `EXCALIDRAW_BACKEND_HOST` is unset (the default), `trusted_fallback_host` accepts any request `Host` header whose name parses as loopback, RFC 1918, or link-local IPv4 (`is_trusted_lan_host`, frontend.rs:140-150). The check validates only the header *value*, not the actual peer — an unauthenticated remote client can send `Host: 10.0.0.1`, `Host: 10.0.0.2:1`, … (~17.9M private IPv4 values × 65535 ports), and every distinct value (a) triggers a full `spawn_blocking` render of a multi-MB bundle and (b) inserts a multi-MB `Bytes` entry into the cache that is never evicted. The doc comment's claim "Bounded by embedded text assets × trusted hosts" is effectively unbounded because "trusted hosts" is a set of tens of millions of header values.

**Why it matters**: Unauthenticated memory exhaustion (and CPU amplification on each miss) on the default configuration (`LISTEN=0.0.0.0`, no `EXCALIDRAW_BACKEND_HOST`). A few thousand requests with varied private-IP Host headers can grow the process by gigabytes. The sibling in-memory stores in `firebase.rs` already solve exactly this class with `BoundedMap` (entry + byte caps, LRU eviction, CONC-7-friendly locking) — the rewrite cache predates that hardening (introduced when ASYNC-1/task-0033 moved rendering off the executor) and never got a bound.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 RewriteCache enforces an entry-count and/or total-byte bound (e.g. reuse BoundedMap from firebase.rs) so unbounded distinct Host values cannot grow memory without limit
- [x] #2 A test demonstrates that inserting past the bound evicts rather than grows the cache
- [x] #3 Render work for untrusted/over-bound keys does not bypass the bound (no cache-as-side-effect growth)
<!-- AC:END -->
