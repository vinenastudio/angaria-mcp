# Releasing

1. Set the same version in `Cargo.toml`, `manifest.json` and `server.json` (both `version` fields and the release URL in `packages[0].identifier`).
2. Run `./scripts/build.sh`. It prints the SHA-256 of `dist/angaria-mcp.mcpb`.
3. Put that value in `server.json` as `packages[0].fileSha256`. Until the first release it holds 64 zeros, a placeholder that is not a real hash.
4. Tag the commit (`v0.1.0`) and attach `dist/angaria-mcp.mcpb` to the GitHub release under exactly that file name.
5. Publish `server.json` to the MCP Registry with `mcp-publisher`, logged in with the DNS method for `angaria.app`.
