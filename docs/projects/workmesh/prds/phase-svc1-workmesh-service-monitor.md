# WorkMesh PRD: Phase SVC1 - LAN-Safe Monitoring Service

Date: 2026-02-18
Owner: Luis Lobo
Status: Implemented

## Problem

WorkMesh tracks high-value state across sessions/workstreams/worktrees, but visibility is fragmented across CLI/MCP commands and individual terminals.
A central ad-hoc monitor is needed to quickly review active work and recover flow after interruptions.

## Goals

- Provide a standalone monitoring service (`workmesh-service`) independent from CLI/MCP orchestration.
- Offer a browser UI for desktop and phone on local LAN.
- Expose a stable read API (`/api/v1/*`) for future integrations.
- Include realtime updates with graceful fallback.

## Non-goals (SVC1)

- No task/workstream/session mutation endpoints.
- No chat transcript ingestion.
- No remote command execution / shell control.

## Requirements

### Runtime and access

- New binary: `workmesh-service`.
- Local and LAN binding via `--host`/`--port`.
- Non-loopback bind requires token auth.

### Auth model

- Accept bearer token for API clients.
- Support browser login flow that establishes auth cookie.
- Protect UI, API, and WebSocket when token mode is enabled.

### UI pages

- Dashboard
- Sessions
- Workstreams
- Worktrees
- Repos

### API

- `GET /api/v1/summary`
- `GET /api/v1/sessions`
- `GET /api/v1/sessions/{id}`
- `GET /api/v1/workstreams`
- `GET /api/v1/workstreams/{id}`
- `GET /api/v1/worktrees`
- `GET /api/v1/repos`

### Realtime

- WebSocket endpoint `/ws`.
- Event types: `snapshot`, `delta`, `heartbeat`.
- Browser fallback to polling on websocket failure.

### Data sources

Read from existing WorkMesh state:
- sessions events/current
- workstreams registry
- worktrees registry
- per-repo worktree reconciliation (best effort)

## Acceptance Criteria

- Service runs as standalone crate in workspace (`crates/workmesh-service`).
- LAN access works with token auth from phone browser.
- UI and API surface sessions/workstreams/worktrees/repos with deterministic ordering.
- `/ws` updates UI flow and polling fallback activates when needed.
- Read failures are surfaced as warnings; server remains available.
- README/README.json/docs are updated and linked.
