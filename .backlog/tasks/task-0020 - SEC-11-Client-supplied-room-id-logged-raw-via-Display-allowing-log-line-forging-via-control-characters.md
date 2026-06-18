---
id: TASK-0020
title: >-
  SEC-11: Client-supplied room id logged raw via Display, allowing log-line
  forging via control characters
status: Done
assignee:
  - TASK-0025
created_date: '2026-06-07 12:04'
updated_date: '2026-06-07 12:52'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
ordinal: 20000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/collab/src/lib.rs:173` (`debug!(%sid, %room, count, "join-room")`), `crates/collab/src/lib.rs:145` (`warn!(sid = %sid, room, event, ...)` in `log_emit_failure`)

**What**: The room id is attacker-controlled (unauthenticated socket, any bytes up to `MAX_ROOM_ID_BYTES` = 256 on the join path, unbounded on the broadcast path per TASK-0016) and is interpolated into tracing events via `Display` without sanitization. With the default `tracing_subscriber::fmt` plain-text writer, embedded `\n`/`\r` and ANSI escapes pass through verbatim, letting a client forge log lines or corrupt terminal output (CWE-117). Notably the rejected-join path at lib.rs:165-169 already avoids this by logging only `room_id_len` — the success path and `log_emit_failure` lost that care.

**Why it matters**: Forged log entries can mask abuse or inject misleading audit trails in exactly the component that is unauthenticated and internet-facing. Low severity because JSON-formatted subscribers escape the value and no secrets are involved; fix is cheap (log with `?room` Debug-escaping, or sanitize/truncate control characters once at intake).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Client-supplied room ids are logged Debug-escaped (?room) or sanitized of control characters in all tracing calls in crates/collab
- [x] #2 A unit test (extending the existing failed_emit_is_logged_with_context capture harness) shows a room id containing a newline cannot produce a second log line
<!-- AC:END -->
