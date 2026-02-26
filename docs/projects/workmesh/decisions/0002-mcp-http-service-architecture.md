# Decision: MCP-over-HTTP service architecture

Date: 2026-02-26
Status: Accepted

## Context
Phase 6 introduces a long-lived service mode so WorkMesh can:
- expose MCP tool execution over HTTP
- support local/LAN operational access
- host multiple tool domains in one runtime
- allow safe runtime reloads without full process restart

Existing behavior in `workmesh` (CLI) and `workmesh-mcp` must remain compatible.

## Decision
Adopt a dedicated `workmesh-service` runtime with a transport adapter that maps HTTP
requests to the same core tool handlers used by MCP/CLI.

Service architecture:
- `config`: load/validate service config snapshot
- `server`: HTTP listener, middleware, graceful shutdown
- `transport::mcp_http`: MCP-style invocation contract over HTTP
- `toolhost`: provider registry and namespaced dispatch
- `auth`: token policy and LAN safeguards
- `observability`: request IDs, metrics, health/status

Core logic remains in `workmesh-core`; service mode is an orchestration/transport layer.

## Contract (v1 draft)

### Endpoint: `GET /v1/healthz`
- Purpose: process liveness
- Response: `200 {\"ok\":true,\"service\":\"workmesh-service\",\"version\":\"...\"}`

### Endpoint: `GET /v1/readyz`
- Purpose: readiness for request handling
- Response: `200` when config/provider registry loaded; `503` otherwise

### Endpoint: `GET /v1/providers`
- Purpose: discover loaded providers/capabilities
- Response includes provider namespace, version, and tool list metadata

### Endpoint: `POST /v1/mcp/invoke`
- Purpose: invoke a tool via MCP-over-HTTP transport
- Request body:
```json
{
  "request_id": "optional-client-id",
  "namespace": "workmesh",
  "tool": "list_tasks",
  "arguments": {
    "root": ".",
    "status": "To Do"
  }
}
```
- Success response:
```json
{
  "request_id": "optional-client-id",
  "ok": true,
  "result": { "..." : "tool output" },
  "meta": { "provider": "workmesh", "tool": "list_tasks" }
}
```
- Error response:
```json
{
  "request_id": "optional-client-id",
  "ok": false,
  "error": {
    "code": "INVALID_ARGUMENT|NOT_FOUND|CONFLICT|UNAUTHORIZED|INTERNAL",
    "message": "human-readable summary",
    "details": { "..." : "optional" }
  }
}
```

## Error mapping policy
- Validation/input errors -> `400 INVALID_ARGUMENT`
- Unknown provider/tool -> `404 NOT_FOUND`
- State/version conflicts -> `409 CONFLICT`
- Auth failures -> `401/403 UNAUTHORIZED`
- Unexpected failures -> `500 INTERNAL`

Transport must preserve deterministic, actionable error messages already present in
CLI/MCP behavior.

## Runtime reload model
- Config/provider registry represented as versioned in-memory snapshot.
- Reload performs atomic snapshot swap after validation.
- In-flight requests continue on prior snapshot; new requests use latest snapshot.
- Reload failures do not alter active snapshot.

## Security baseline
- Default bind: `127.0.0.1`.
- LAN bind requires explicit opt-in config.
- Non-localhost access requires bearer token auth by default.
- Request body size and timeout limits enforced at middleware.

## Consequences
- Enables a single runtime to serve multiple tool domains.
- Preserves parity by sharing core handlers instead of duplicating business logic.
- Creates a stable platform for future remote workflows (including phone-driven use).
