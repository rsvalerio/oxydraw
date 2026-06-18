---
id: TASK-0129
title: >-
  DUP-2: parseShareFragment and parseRoomFragment duplicate the #prefix=<a>,<b>
  fragment-parsing logic
status: Done
assignee:
  - TASK-0142
created_date: '2026-06-16 18:09'
updated_date: '2026-06-16 20:18'
labels:
  - code-review-web
  - duplication
dependencies: []
priority: low
ordinal: 129000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `frontend/src/share.ts:63-75` (`parseShareFragment`) and `frontend/src/collab/protocol.ts:33-45` (`parseRoomFragment`)

**What**: Both functions are structurally identical: default `hash` to `window.location.hash`, bail unless it starts with a prefix (`#json=` / `#room=`), `slice` past the prefix, `split(",")` into two parts, and return `null` unless both parts are truthy. Only the prefix and the returned field names differ.

**Why it matters**: Two copies of the same capability-URL parsing rule drift independently — a hardening fix (e.g. rejecting extra commas, length-bounding the id/key, trimming) applied to one will be missed on the other. Extract a shared `parseFragment(prefix, hash)` returning `[a, b] | null` and have both call it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A single shared helper parses the #prefix=<a>,<b> fragment form; parseShareFragment and parseRoomFragment delegate to it
- [ ] #2 Existing parse behavior (including the null cases) is preserved
<!-- AC:END -->
