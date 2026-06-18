---
id: TASK-0132
title: >-
  TEST-5: security-critical crypto and capability-URL parsing have no test
  coverage
status: Done
assignee:
  - TASK-0142
created_date: '2026-06-16 18:10'
updated_date: '2026-06-16 20:18'
labels:
  - code-review-web
  - tests
dependencies: []
priority: medium
ordinal: 132000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `frontend/src/crypto.ts` (AES-GCM encrypt/decrypt, base64url round-trip), `frontend/src/share.ts` + `frontend/src/collab/protocol.ts` (fragment parsing). No `*.test.ts(x)` files exist in `src/` (vitest runs with `passWithNoTests`).

**What**: The end-to-end-encryption primitives and the capability-URL parsers — the modules the whole share/collab security model rests on — have zero unit tests.

**Why it matters**: These units have exactly the properties TEST-5/TEST-6 flag as must-test: crypto correctness and untrusted-input parsing. A regression in `bytesToBase64Url`/`base64UrlToBytes` (e.g. padding or `+//-_` substitution), in the `iv || ciphertext` framing of `encrypt`/`decrypt`, or in `parseShareFragment`/`parseRoomFragment` rejection of malformed input would silently corrupt keys or accept bad fragments, and nothing would catch it. Add focused tests: encrypt→decrypt round-trip, fresh-IV-per-call, base64url round-trip over binary, and parser accept/reject cases.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 crypto.ts has tests covering encrypt/decrypt round-trip, distinct IV per encryption, and base64url round-trip over arbitrary bytes
- [ ] #2 parseShareFragment and parseRoomFragment have tests for valid input and each malformed/rejected case
- [ ] #3 Tests run green under the existing vitest setup
<!-- AC:END -->
