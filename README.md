# WorkMesh

WorkMesh is a docs-first project and task orchestration system for developers and coding agents.

It keeps planning state next to the code:
- tasks in Markdown
- repo-local context in `workmesh/context.json`
- durable decisions in `workmesh/truth/`
- global continuity in `~/.workmesh/`

## Install

Verify installed binaries:

```bash
workmesh --version
workmesh-mcp --version
```

Build from source:

```bash
git clone git@github.com:luislobo/workmesh.git
cd workmesh
cargo build -p workmesh
cargo build -p workmesh-mcp
```

Codex MCP example:

```toml
[mcp_servers.workmesh]
command = "/usr/local/bin/workmesh-mcp"
args = []
```

## Documentation

Primary documentation:
- [`docs/README.md`](docs/README.md)

Supporting references:
- [`docs/architecture.md`](docs/architecture.md)
- [`docs/reference/commands.md`](docs/reference/commands.md)
- [`docs/setup/run-modes-and-agent-mcp.md`](docs/setup/run-modes-and-agent-mcp.md)
- [`CHANGELOG.md`](CHANGELOG.md)

Agent-readable mirror:
- [`README.json`](README.json)
