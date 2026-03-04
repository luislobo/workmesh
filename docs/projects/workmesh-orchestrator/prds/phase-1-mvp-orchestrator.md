# PRD: Phase 1 - WorkMesh Orchestrator MVP (Hub + Worker + GitHub)

Date: 2026-02-26
Owner: Luis Lobo
Status: Draft

## Summary
Build a Stripe-style devbox orchestration system on Proxmox with one VM per workstream. A central hub provisions devboxes, tracks workstreams, and proxies MCP calls to per-VM workers. Workers clone GitHub repos locally and run a local WorkMesh runtime (stdio MCP adapter or embedded workmesh-core) for repo-local writes. The MVP focuses on reliable orchestration and visibility, not unattended CI loops.

## Problem
Managing many customer codebases in parallel with agents requires:
- isolated environments per workstream/branch
- a central view to switch contexts quickly
- safe local writes to each repo without filesystem sharing

WorkMesh alone provides local task/state management, but not cross-host orchestration or a central control plane.

## Goals
- Provision Proxmox VMs per workstream (one devbox per branch).
- Central hub tracks customers, repos, workstreams, sessions, and VM status.
- Workers handle all repo-local writes via a local WorkMesh runtime.
- Hub exposes a unified MCP proxy endpoint per workstream.
- Basic UI for visibility and fast switching.

## Non-goals (Phase 1)
- No full unattended CI loop (no "minion" blueprint yet).
- No GitLab integration (GitHub only in MVP).
- No mobile UI.
- No multi-region or cloud provisioning.

## Architecture

### Components
1. Hub (Control Plane)
   - API + UI
   - Proxmox provisioner
   - Registry database (Postgres)
   - MCP proxy router
   - Event collection

2. Worker (per devbox VM)
   - GitHub clone + checkout
   - local WorkMesh runtime with `WORKMESH_REPO_ROOT`
   - Worker daemon (gRPC to hub)

3. GitHub
   - Source of truth for repos and branches

### High-level flow
1. User creates workstream in hub (customer + repo + branch).
2. Hub clones VM from template and starts it.
3. Worker registers to hub and clones the GitHub repo.
4. Worker starts the local WorkMesh runtime.
5. Hub exposes `/mcp/<customer>/<repo>/<workstream>` proxy to worker `/mcp`.

## Data Model (Hub)
- Customer
  - Repo
    - Workstream
      - Devbox VM
      - Sessions
      - Status

## Public Interfaces

### Hub HTTP API (MVP)
- `POST /v1/customers`
- `POST /v1/repos`
- `POST /v1/workstreams`
- `GET /v1/workstreams`
- `GET /v1/workstreams/{id}/status`
- `POST /v1/workstreams/{id}/pause|resume|destroy`

### Hub MCP Proxy
- `/mcp/<customer>/<repo>/<workstream>` -> worker `/mcp`

### Worker gRPC API (Hub -> Worker)
- `RegisterWorker`
- `CloneRepo(repo_url, branch)`
- `StartWorkmesh(repo_root)`
- `StopWorkmesh()`
- `ExecuteTool(mcp_request)`
- `StreamEvents()`

## MVP Scope (GitHub)
- GitHub tokens stored per customer.
- Repo cloning via HTTPS.
- Workstreams map to Git branches.

## Implementation Plan (Phase 1)

### A. Hub Core
- Define registry schema (customers, repos, workstreams, workers).
- Implement Workstream API (create/list/status).
- Proxmox client for VM clone/start/stop.
- MCP proxy routing.

### B. Worker Core
- Bootstrap agent that registers with hub.
- GitHub clone + checkout.
- Start the local WorkMesh runtime with repo root.

### C. UI
- Workstreams list view.
- Status + repo + branch display.
- Link to MCP endpoint per workstream.

## Testing
- Provision VM -> worker registers within 60s.
- Repo clone succeeds and the local WorkMesh runtime starts.
- MCP tool call routed through hub returns success.
- Workstreams list reflects accurate status.

## Acceptance Criteria
- One command creates a workstream and returns a live MCP endpoint.
- All repo writes occur on the worker VM.
- Hub shows multi-customer workstreams centrally.
- Switching between workstreams takes <10s.

## Assumptions
- GitHub is the source control system for MVP.
- One workstream equals one VM.
- Proxmox templates are available.
- Token-based auth for hub and worker communication.

## Future Phases (Post-MVP)
- GitLab integration.
- Blueprint-based unattended automation.
- CI loop integration (lint/test/PR creation).
- Advanced UI (live agent step viewer).
