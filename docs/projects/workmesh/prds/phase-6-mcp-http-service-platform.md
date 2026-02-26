# PRD: Phase 6 - MCP HTTP Service and Extensible Tool Platform

Date: 2026-02-26
Owner: Luis Lobo
Status: Planned

## Problem
WorkMesh currently provides a CLI and an MCP server process, but it lacks a long-lived HTTP service mode for:
- live updates without restarting every agent process
- exposing a stable endpoint over local network/LAN
- hosting multiple tool domains under one runtime (not only task management)

This limits real-time operations, central observability, and future phone-first remote interaction.

## Goals
- Add an HTTP service mode for WorkMesh with MCP-compatible request handling.
- Support hot updates for service config/tool wiring without full process restarts.
- Make WorkMesh a unified local agent development platform that can host multiple tool domains.
- Preserve CLI/MCP parity for existing WorkMesh operations.
- Keep local-first behavior and safe LAN defaults.

## Non-goals (Phase 6)
- No forced cloud integration.
- No hard dependency on external databases.
- No mobile UI in this phase (only API/service groundwork).
- No breaking changes to existing CLI/MCP command behavior.

## Requirements

### Service runtime
- New `workmesh-service` binary/crate.
- Long-lived HTTP server with graceful startup/shutdown.
- Configurable bind address/port and data root.
- Health/readiness endpoints.

### MCP over HTTP
- HTTP endpoint(s) that can execute existing WorkMesh tools via a transport adapter.
- Deterministic error mapping and stable JSON responses.
- Backward compatibility with current MCP tool semantics.

### Extensible multi-tool host
- Introduce a tool-host abstraction:
  - core WorkMesh tool provider
  - additional providers/modules can be registered
- Route requests by tool namespace/capability.
- Standardized metadata surface for discoverability.

### Hot updates
- Reload configuration/provider registry at runtime via:
  - explicit admin reload endpoint/command
  - optional file-watch trigger (safe and bounded)
- No in-flight request corruption during reload.

### Security for LAN use
- Default localhost binding.
- Optional LAN binding with explicit enablement.
- Token-based auth for non-localhost traffic (minimum baseline).
- Request size/time limits and simple rate limiting.

### Observability and operations
- Structured logs with request IDs.
- Endpoint-level metrics.
- Service status snapshot: loaded providers, config version, uptime, active requests.

### Tooling parity and ergonomics
- CLI support to run/manage service mode (`workmesh service ...`).
- Clear diagnostics when service mode is unavailable/misconfigured.
- No regression in existing `workmesh` and `workmesh-mcp` flows.

## Architecture outline

### Crate structure (target)
- `crates/workmesh-service`
  - `config` (load/validate/reload)
  - `server` (http runtime, middleware, lifecycle)
  - `transport::mcp_http` (request mapping and response shaping)
  - `toolhost` (provider registry + dispatch)
  - `auth` (token and policy checks)
  - `observability` (logs/metrics/health)

### Core integration
- Reuse `workmesh-core` for domain logic and storage guarantees.
- Reuse or adapt existing `workmesh-mcp` tool handlers via a shared interface to avoid logic duplication.

### Data and concurrency
- Follow existing storage primitives (`storage.rs`) for all tracking writes.
- Ensure service-level parallel requests do not bypass locking/atomic guarantees.

## Delivery phases

1. Foundation
- Service crate scaffold, config model, health/readiness endpoints, CLI wiring.

2. Transport
- MCP-over-HTTP request/response adapter with parity tests.

3. Extensibility
- Provider registry and namespaced dispatch for multi-domain tools.

4. Runtime reload
- Safe reload path for config/provider changes.

5. Security and operations
- Auth baseline, limits, metrics, structured logs, admin introspection.

6. Docs and rollout
- Local/LAN setup docs, migration guidance, troubleshooting, test coverage.

## Acceptance criteria
- WorkMesh can run as a long-lived HTTP service on localhost and optional LAN.
- Existing WorkMesh tool operations are reachable through MCP-over-HTTP with parity behavior.
- Tool-host can register at least one additional non-task provider in a stable way.
- Config/provider reload works without full process restart and without losing storage integrity.
- Security and observability baseline is in place for local network usage.
- Documentation includes setup, operations, and rollback guidance.

## Risks and mitigations
- Risk: duplicated business logic across CLI/MCP/service.
  - Mitigation: shared handler interfaces and core-first orchestration.
- Risk: reload race conditions.
  - Mitigation: versioned config snapshot + atomic swap + in-flight request isolation.
- Risk: LAN exposure and unsafe defaults.
  - Mitigation: localhost default, explicit LAN opt-in, token auth requirement for non-localhost.
- Risk: scope creep into full platform too early.
  - Mitigation: phase gates and explicit non-goals.

## Open decisions
- Exact HTTP protocol shape for MCP transport (single endpoint vs routed endpoints).
- Provider packaging model (compiled-in registry first, plugin model later).
- Default auth behavior for trusted local subnet vs strict token-only.
- Metrics format/export target (Prometheus text vs JSON-only in first cut).
