# WorkMesh agent notes

## Docs sync rule
- `README.md` (humans) and `README.json` (agents) must be kept in sync.
- If you change install, quickstart, MCP setup, commands, layout, or roadmap: update both files in the same commit.
- `CHANGELOG.md` must be updated whenever a new version is cut (version bump + tag).

## Versioning policy (Rust-standard)
- `Cargo.toml` holds the SemVer release version (manual bumps for releases).
- When releasing a new version:
  - Move relevant items from `CHANGELOG.md` `[Unreleased]` into a new `## [X.Y.Z] - YYYY-MM-DD` section.
  - Ensure the compare links at the bottom of `CHANGELOG.md` are updated.
  - Then bump `Cargo.toml` + tag.
- Binaries include automatic build metadata in `--version`:
  - `X.Y.Z+git.<commit_count>.<sha>[.dirty]`
  - Example: `0.2.0+git.123.abc1234.dirty`
- This makes every committed build identifiable without auto-editing `Cargo.toml` on each compile.

## Common commands
- Build CLI: `cargo build -p workmesh`
- Build MCP: `cargo build -p workmesh-mcp`
- Tests: `cargo test -p workmesh-core && cargo test -p workmesh-mcp`
