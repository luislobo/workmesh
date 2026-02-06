# WorkMesh agent notes

## Versioning policy (Rust-standard)
- `Cargo.toml` holds the SemVer release version (manual bumps for releases).
- Binaries include automatic build metadata in `--version`:
  - `X.Y.Z+git.<commit_count>.<sha>[.dirty]`
  - Example: `0.2.0+git.123.abc1234.dirty`
- This makes every committed build identifiable without auto-editing `Cargo.toml` on each compile.

## Common commands
- Build CLI: `cargo build -p workmesh`
- Build MCP: `cargo build -p workmesh-mcp`
- Tests: `cargo test -p workmesh-core && cargo test -p workmesh-mcp`

