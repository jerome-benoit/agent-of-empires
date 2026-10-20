## Description

Experimental Daemon-owned session state for the TUI and web dashboard. Clients consume revisioned snapshots and submit commands instead of competing with server-side lifecycle writers.

This draft is evidence to mine under #2734, not an official modularization-wave delivery. Ticket reconciliation awaits human review in #4505.

- Return epoch/revision mutation receipts. The TUI waits for a same-epoch snapshot at or beyond the receipt. The TUI hides unavailable rows and disables new native interaction while disconnected. Unknown TUI mutation outcomes block conflicting actions without local fallback or automatic replay; reconnect is explicit and requires a fresh snapshot.
- Route creation, lifecycle, attach/send, paired terminals/tools, metadata and ordering through the runtime. Reservations serialize conflicting operations. Native interaction requires the acknowledged generation and exact live target; terminal handoff retires the matching ACP runner. Archived or trashed sessions cannot launch or resume.
- Commit groups, profiles, projects and settings against the canonical physical store and selected configuration layer. Preserve omitted, explicit-false and null-to-inherit overrides. Fence stale alias results after store replacement. Bind hook/MCP approvals to reviewed configuration and deduplicate keyed creation retries. Preserve conversation identity through import, fork and resume, and announce lost context before fresh conversation assignment.
- Keep cleanup Daemon-owned after row removal. Match process births, worker generations and exact owned systemd units; protect borrowed resources and replacement transcripts. Retain cleanup authority until physical retirement and durable acknowledgement. Metadata-only removal requires a matching failed forced purge and retains payload files. Trash lifecycle UUIDs reject stale retries.
- Derive local-owner authority from Unix peer credentials, never TCP loopback or client claims. Restrict secret-bearing output and native interaction to negotiated capabilities and matching local namespace/tmux context. Preserve read-only and CityHall policy, trusted-ingress checks, and separate password/passkey rate budgets. Require HTTPS for credential-bearing native HTTP outside loopback; disable redirects and bypass local proxies. Same-UID processes remain trusted; this is not Daemon-instance authentication.
- Support a Core-only runtime without dashboard assets or a TCP listener. Native protocol 2 uses `/api/runtime/ws` and private `daemon/api.sock`; the web server also mounts the route, but TCP peers never gain owner authority or native interaction. Preserve the published CLI read protocol 4 at `/api/runtime/read/ws` and `runtime.sock`. Serve MCP from the same canonical session/ordering snapshot, disabled by default. Use versioned migrations for persisted contract changes.

**API compatibility:** `GET /api/sessions` returns `{ sessions, workspace_ordering }` with runtime epoch/revision headers, not a bare array. Selected-trash purges require timestamp, generation and `expected_trash_lifecycle_id` together.

## PR Type

- [x] New Feature
- [x] Bug Fix
- [x] Refactor
- [x] Documentation
- [ ] Infrastructure / CI

## Checklist

- [x] New and existing tests pass
- [x] Documentation was updated where necessary
- [x] For UI changes: included screenshot or recording

## Test Coverage Analysis

**Web dashboard / structured view (Playwright user story)**
- [x] Added or updated a Playwright spec under `web/tests/` and updated `web/tests/coverage-matrix.json`

**TUI / CLI (e2e test)**
- [x] Added or updated an e2e test under `tests/e2e/`

Coverage includes receipt reconciliation, authority and offline boundaries, lifecycle contention, provisioning/import defaults, purge/resource retention, migration provenance and reload persistence. Actual runtime probes exercise terminal-status freshness, same-pane replacement while disconnected, explicit reconnect, existing-worktree creation, MCP policy and frozen read-protocol compatibility. Scripted ACP agents and virtual WebAuthn credentials are not cloud-provider or physical-hardware validation.

[Exact-head checks](https://github.com/agent-of-empires/agent-of-empires/pull/4009/checks) · [Per-finding verdicts, commands and visual evidence](https://github.com/agent-of-empires/agent-of-empires/pull/4009#issuecomment-6055738768).

## Related issues

Refs #2734, #3835, #3836, #3837, #3841, #3896, #3897, #3898, #3899, #3900, #3901, #3902 and #4505. No issue closure is requested.

## AI Usage

- [x] AI was used

**AI Model/Tool used:** Claude, OpenAI Codex and Oh My Pi

**Any Additional AI Details you'd like to share:** AI-assisted analysis, code changes and verification. Human scope review is pending in #4505.

- [x] I am an AI Agent filling out this form (check box if true)
