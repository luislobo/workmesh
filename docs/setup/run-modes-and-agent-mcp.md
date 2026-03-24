# Run Modes and Agent MCP Setup

This document defines how to run WorkMesh and how to configure agents for:
- CLI mode
- MCP stdio mode

## Mode summary

| Mode | Binary | Transport | Best for |
|---|---|---|---|
| CLI | `workmesh` | local process | direct shell usage and scripts |
| MCP stdio | `workmesh-mcp` | stdio | coding agents that support MCP command processes |

## Install

Prebuilt binaries:
```bash
workmesh --version
workmesh-mcp --version
```

Build from source:
```bash
cargo build -p workmesh
cargo build -p workmesh-mcp
```

## CLI mode

Run commands directly:
```bash
workmesh --root . list --json
workmesh --root . next --json
workmesh --root . bootstrap --project-id <project-id> --feature "<feature>" --json
workmesh --root . render table --data '[{"task":"task-001","status":"Done"}]'
```

CLI render fallback:
- `workmesh --root . render table|kv|stats|list|progress|tree|diff|logs|alerts|chart-bar|sparkline|timeline`
- input via `--data`, `--data-file`, or `--stdin`
- optional renderer settings via `--configuration` or `--config-file`

## MCP stdio mode

Run MCP server process:
```bash
workmesh-mcp
```

Health/version check:
```bash
workmesh-mcp --version
```

Render tools (MCP stdio):
- `render_table`, `render_kv`, `render_stats`, `render_list`, `render_progress`
- `render_tree`, `render_diff`, `render_logs`, `render_alerts`
- `render_chart_bar`, `render_sparkline`, `render_timeline`

Each render tool accepts:
- `data`: required JSON-encoded string payload
- `configuration`: optional typed object
- `format`: optional, used by `render_table`

For backward compatibility, MCP still accepts native JSON values for `data`, but agent/tool integrations should send the explicit JSON string form.

Mutation response policy (MCP stdio):
- mutation tools return minimal acknowledgements by default to save tokens
- pass `verbose=true` when you need richer post-write state in the same call
- prefer read tools (`show_task`, `truth_show`, `session_show`, `workstream_show`, `context_show`) when full objects are needed

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

If your agent only supports HTTP MCP, WorkMesh currently only supports stdio.
