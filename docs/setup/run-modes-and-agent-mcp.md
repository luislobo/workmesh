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

Render tools (MCP stdio):
- `render_table`, `render_kv`, `render_stats`, `render_list`, `render_progress`
- `render_tree`, `render_diff`, `render_logs`, `render_alerts`
- `render_chart_bar`, `render_sparkline`, `render_timeline`

Each render tool accepts `data` plus optional `format` and `configuration`, and returns rendered text.

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
