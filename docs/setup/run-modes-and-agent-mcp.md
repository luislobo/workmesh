# Run Modes and Agent MCP Setup

This document defines how to run WorkMesh and how to configure agents for:
- CLI mode
- MCP stdio mode
- MCP HTTP mode

## Mode summary

| Mode | Binary | Transport | Best for |
|---|---|---|---|
| CLI | `workmesh` | local process | direct shell usage and scripts |
| MCP stdio | `workmesh-mcp` | stdio | coding agents that support MCP command processes |
| MCP HTTP | `workmesh-service` | HTTP (`/v1/mcp/invoke`) | long-lived local/LAN service and custom GUI integrations |

## Install

Prebuilt binaries:
```bash
workmesh --version
workmesh-mcp --version
workmesh-service --version
```

Build from source:
```bash
cargo build -p workmesh
cargo build -p workmesh-mcp
cargo build -p workmesh-service
```

## CLI mode

Run commands directly:
```bash
workmesh --root . list --json
workmesh --root . next --json
workmesh --root . bootstrap --project-id <project-id> --feature "<feature>" --json
```

## MCP stdio mode

Run MCP server process:
```bash
workmesh-mcp
```

Health/version check:
```bash
workmesh-mcp --version
```

### Agent configuration: CLI clients

Codex (`~/.codex/config.toml`):
```toml
[mcp_servers.workmesh]
command = "/usr/local/bin/workmesh-mcp"
args = []
```

Generic CLI agent config (JSON shape):
```json
{
  "mcpServers": {
    "workmesh": {
      "command": "/usr/local/bin/workmesh-mcp",
      "args": []
    }
  }
}
```

### Agent configuration: GUI clients

For GUI apps that support command-based MCP servers, configure:
- command: absolute path to `workmesh-mcp`
- args: `[]`
- optional environment variables (if needed by your GUI host)

If the GUI has an MCP settings page, use the same command/args pair there.

## MCP HTTP mode

Start service (foreground):
```bash
workmesh --root . service start --host 127.0.0.1 --port 4747
```

LAN-safe start:
```bash
workmesh --root . service start --host 0.0.0.0 --port 4747 --auth-token "<token>"
```

Persistent startup with systemd:
```bash
workmesh --root . service install-systemd --scope user --enable --start
```

Probe service:
```bash
curl -s http://127.0.0.1:4747/v1/healthz
curl -s http://127.0.0.1:4747/v1/readyz
```

Invoke tool over HTTP:
```bash
curl -s \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  http://127.0.0.1:4747/v1/mcp/invoke \
  -d '{"namespace":"workmesh","tool":"list_tasks","arguments":{"root":"."}}'
```

Provider namespaces in HTTP mode:
- `workmesh`: task/context/truth/workstream/worktree/session tools
- `system`: service diagnostics (`ping`, `version`, `status`)
- `render`: native Rust render tools (`render_table`, `render_kv`, `render_stats`, `render_progress`, `render_tree`, `render_diff`, `render_logs`, `render_alerts`, `render_list`, `render_chart_bar`, `render_sparkline`, `render_timeline`)

Invoke a render tool over HTTP:
```bash
curl -s \
  -H "Authorization: Bearer <token>" \
  -H "Content-Type: application/json" \
  http://127.0.0.1:4747/v1/mcp/invoke \
  -d '{"namespace":"render","tool":"render_table","arguments":{"data":[{"name":"api","status":"ok"},{"name":"worker","status":"degraded"}]}}'
```

### Agent configuration: HTTP-capable clients

For agents/GUI tools that can call HTTP tools directly, configure:
- base URL: `http://<host>:4747`
- invoke endpoint: `/v1/mcp/invoke`
- auth header: `Authorization: Bearer <token>` for protected setups

Renderer note:
- `render` namespace is provided by `workmesh-service` (MCP HTTP mode).
- External Node `mcp-gui` is retired as the primary renderer workflow.

If your agent only supports stdio MCP (not HTTP tool endpoints), use MCP stdio mode.

## Containers

Sample files:
- `docker/workmesh-service/Dockerfile`
- `docker/workmesh-service/docker-compose.yml`
- `docker/workmesh-service/service.toml.example`

Quick start:
```bash
cd docker/workmesh-service
WORKMESH_REPO_ROOT=/abs/path/to/repo \
WORKMESH_AUTH_TOKEN=<token> \
docker compose up --build -d
```

## Security baseline

- Keep default bind on localhost unless LAN access is required.
- When using non-localhost bind, always set an auth token.
- For access outside trusted LAN, place service behind TLS reverse proxy.
