---
id: TASK-0125
title: >-
  RT-1: Collab inbound broadcast payload shape is not validated before applying
  to scene/presence
status: Done
assignee:
  - TASK-0142
created_date: '2026-06-16 18:09'
updated_date: '2026-06-16 20:18'
labels:
  - code-review-web
  - realtime
dependencies: []
priority: medium
ordinal: 125000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `frontend/src/collab/Collab.ts:197-218` (`onClientBroadcast`), `:288-302` (`applyPointer`)

**What**: Decryption + `JSON.parse` are wrapped in try/catch (good), but the resulting object is cast `as BroadcastPayload` and switched on `message.type` with no shape validation. `MOUSE_LOCATION` payloads (`socketId`, `pointer`, `selectedElementIds`, `username`) are written straight into the collaborators map and `api.updateScene`. `SCENE_*` `payload.elements` are passed to `applyRemoteElements`.

**Why it matters**: Broadcast frames are AES-GCM authenticated, so an injector must already hold the room key — but any peer (the room link is the only capability) is therefore implicitly trusted. A buggy or hostile peer can send a frame whose `type` matches but whose `payload` is malformed (missing `pointer`, non-string `username`, wrong `elements` type), crashing the handler or polluting presence/scene state. The scene path is partially mitigated because `restoreElements` sanitizes element input; the `MOUSE_LOCATION` path has no such guard. Per RT-1 / the project's "never trust data crossing a boundary" philosophy, reject malformed payloads instead of applying them.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 onClientBroadcast validates message shape (type + required payload fields) before dispatch and drops malformed frames (logging at most a generic warning, never the plaintext)
- [ ] #2 MOUSE_LOCATION payload fields are validated/narrowed before being written into the collaborators map
- [ ] #3 A malformed-but-decryptable frame cannot throw out of the handler or inject invalid state
<!-- AC:END -->
