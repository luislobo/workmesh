# WorkMesh Service Reference

`workmesh-service` is a standalone monitoring service for WorkMesh state.
It is independent of CLI/MCP command execution and reads existing WorkMesh files.

## Purpose
- Central view of active sessions, workstreams, worktrees, and repos.
- LAN-accessible dashboard for desktop and phone browsers.
- Stable read API for future integrations.

## Runtime

Build:
```bash
cargo build -p workmesh-service
```

Run (localhost):
```bash
workmesh-service --host 127.0.0.1 --port 4747
```

Run (LAN):
```bash
workmesh-service --host 0.0.0.0 --port 4747 --auth-token "<token>"
```

### Flags
- `--host <ip>`: bind interface (`127.0.0.1` default)
- `--port <u16>`: bind port (`4747` default)
- `--workmesh-home <path>`: override `WORKMESH_HOME`
- `--scan-root <path>`: additional repo roots to include (repeatable)
- `--refresh-ms <u64>`: refresh interval (`3000` default)
- `--auth-token <token>`: access token (required for non-loopback binds)
- `--auth-token-file <path>`: token file
- `--open`: attempt to open browser at startup
- `--json-log`: JSON logs

### Environment
- `WORKMESH_HOME`
- `WORKMESH_SERVICE_TOKEN`

## Security Behavior
- Non-loopback bind without token is rejected at startup.
- Auth is accepted via:
  - `Authorization: Bearer <token>`
  - auth cookie created from `/auth/login`
- UI/API/WebSocket routes require auth when token mode is enabled.

## Web UI Routes
- `GET /`
- `GET /sessions`
- `GET /workstreams`
- `GET /worktrees`
- `GET /repos`
- `GET /login`
- `POST /auth/login`
- `POST /auth/logout`
- `GET /healthz`

## API Routes (`/api/v1`)
- `GET /api/v1/summary`
- `GET /api/v1/sessions`
- `GET /api/v1/sessions/{id}`
- `GET /api/v1/workstreams`
- `GET /api/v1/workstreams/{id}`
- `GET /api/v1/worktrees`
- `GET /api/v1/repos`

## Realtime
- `GET /ws`
- WebSocket emits `snapshot`, `delta`, and `heartbeat` events.
- Browser client auto-falls back to polling if WebSocket fails.

## Data Sources
Global:
- `$WORKMESH_HOME/sessions/events.jsonl`
- `$WORKMESH_HOME/sessions/current.json`
- `$WORKMESH_HOME/workstreams/registry.json`
- `$WORKMESH_HOME/worktrees/registry.json`

Repo-level enrichment:
- discovered repo roots from global records plus `--scan-root`
- git worktree list + registry reconciliation (best effort)

## Behavior Notes
- Service is read-only in this phase.
- Partial/corrupt source reads do not crash the server; warnings are surfaced in summary output.
