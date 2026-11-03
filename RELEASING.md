# Releasing

1. Set the same version in `Cargo.toml`, `manifest.json` and `server.json` (both `version` fields and the release URL in `packages[0].identifier`).
2. Build the universal binary without packing: `SKIP_PACK=1 ./scripts/build.sh`.
3. Sign it with hardened runtime and a timestamp. Run this in a desktop Terminal session so the keychain is reachable:
   `codesign --force --options runtime --timestamp --sign "Developer ID Application: <team name> (<team id>)" dist/angaria-mcp`
4. Notarize it with an App Store Connect API key:
   `ditto -c -k --keepParent dist/angaria-mcp angaria-mcp.zip`
   `xcrun notarytool submit angaria-mcp.zip --key <p8 file> --key-id <key id> --issuer <issuer id> --wait`
   A bare binary cannot be stapled. Gatekeeper checks the ticket online.
5. Pack the bundle the same way `build.sh` does: copy the signed binary into `dist/bundle/server`, add `manifest.json` and `LICENSE`, then run `npx --yes @anthropic-ai/mcpb@2.1.2 validate` and `pack`. Get the SHA-256 with `shasum -a 256 dist/angaria-mcp.mcpb`.
6. Check the binary from the packed bundle: `spctl -a -vv -t install <extracted server/angaria-mcp>` must say "Notarized Developer ID".
7. Put the SHA-256 from step 5 in `server.json` as `packages[0].fileSha256`. Until the first release it holds 64 zeros, a placeholder that is not a real hash.
8. Tag the commit (`v0.1.0`) and attach `dist/angaria-mcp.mcpb` to the GitHub release under exactly that file name.
9. Publish `server.json` to the MCP Registry with `mcp-publisher`, logged in with the DNS method for `angaria.app`.
