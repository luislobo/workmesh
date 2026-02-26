# Phase 6 Service Rollout Validation Runbook

Date: 2026-02-26
Status: Draft

## Scope
Validate service-mode readiness for:
- command-surface diagnostics (`workmesh service verify`)
- lifecycle startup (`workmesh service start ...`)
- HTTP endpoint health and provider discovery
- MCP-over-HTTP invocation behavior
- runtime reload behavior
- rollback to CLI/MCP stdio workflow

## Preconditions
- Build binaries:
  - `cargo build -p workmesh`
  - `cargo build -p workmesh-mcp`
  - `cargo build -p workmesh-service`
- Run tests:
  - `cargo test`

## Local validation (localhost)
1. Verify service binary:
   - `workmesh --root . service verify`
2. Start service (foreground):
   - `workmesh --root . service start --host 127.0.0.1 --port 4747`
3. Health/readiness/status:
   - `curl -s http://127.0.0.1:4747/v1/healthz`
   - `curl -s http://127.0.0.1:4747/v1/readyz`
   - `curl -s http://127.0.0.1:4747/v1/status`
4. Provider catalog:
   - `curl -s http://127.0.0.1:4747/v1/providers`
5. Invoke representative tool call:
```bash
curl -s http://127.0.0.1:4747/v1/mcp/invoke \
  -H 'content-type: application/json' \
  -d '{
    "namespace":"workmesh",
    "tool":"stats",
    "arguments":{"root":"."}
  }'
```

## LAN-safe validation
1. Start with explicit auth token and non-localhost bind:
   - `workmesh --root . service start --host 0.0.0.0 --port 4747 --auth-token "<token>"`
2. Confirm unauthenticated access fails (`401`) for protected routes:
   - `curl -i http://<host>:4747/v1/status`
3. Confirm authenticated access succeeds:
   - `curl -s -H "Authorization: Bearer <token>" http://<host>:4747/v1/status`

## Reload validation
1. Start service with `--config ./service.toml`.
2. Update reload-safe config values (for example `auth_token`).
3. Trigger reload:
   - `curl -s -X POST http://127.0.0.1:4747/v1/admin/reload`
4. Validate response:
   - `ok=true`
   - `config_version` incremented
   - `providers_loaded` non-zero
   - `pending_restart` reflects host/port/body-limit/timeout drift

## Rollback procedure
1. Stop `workmesh-service` process.
2. Continue with established local workflows:
   - CLI: `workmesh --root . <command>`
   - MCP stdio: `workmesh-mcp`
3. If needed, disable service-mode automation while keeping MCP stdio/CLI unchanged.

## Exit criteria
- All local validation checks pass.
- LAN-safe auth behavior is confirmed.
- Reload endpoint returns deterministic status and does not crash service.
- Rollback path is clear and tested.
