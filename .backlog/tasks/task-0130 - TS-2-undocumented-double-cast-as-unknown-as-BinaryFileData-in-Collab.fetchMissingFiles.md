---
id: TASK-0130
title: >-
  TS-2: undocumented double cast 'as unknown as BinaryFileData' in
  Collab.fetchMissingFiles
status: Done
assignee:
  - TASK-0141
created_date: '2026-06-16 18:09'
updated_date: '2026-06-16 19:56'
labels:
  - code-review-web
  - typescript
dependencies: []
priority: low
ordinal: 130000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `frontend/src/collab/Collab.ts:277`

**What**: `this.api.addFiles([{ id, dataURL, mimeType, created: Date.now() } as unknown as BinaryFileData])` force-casts a hand-built object literal through `unknown` to `BinaryFileData`. Unlike the documented brand cast at line 224 (`as unknown as readonly RemoteExcalidrawElement[]`, which carries a comment explaining the missing upstream brand), this cast has no justification and is hiding the real type — `BinaryFileData` likely requires fields beyond the four supplied.

**Why it matters**: `as unknown as` defeats the compiler entirely; if `BinaryFileData` gains or renames a required field, this site won't error and will silently feed the editor an under-specified file object. Construct a properly-typed `BinaryFileData` (filling all required fields) so the type checks, or document why the partial shape is sound the way line 224 does.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The file object is built as a typed BinaryFileData without 'as unknown as', or the cast carries a comment justifying the partial shape
- [x] #2 tsc still passes
<!-- AC:END -->
