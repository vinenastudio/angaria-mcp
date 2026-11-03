#!/usr/bin/env bash
# Builds angaria-mcp for Apple silicon and Intel, joins them into one
# universal binary with lipo, and packs it with manifest.json into
# dist/angaria-mcp.mcpb. Prints the bundle's SHA-256 for server.json.
# Needs macOS, rustup (rust-toolchain.toml installs both targets) and, to
# pack, Node with npx. SKIP_PACK=1 builds the binary only.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
DIST="$ROOT/dist"
BUNDLE="$DIST/bundle"
MCPB_VERSION="2.1.2"

if [[ $(uname -s) != Darwin ]]; then
  echo "build.sh needs macOS (lipo and the Apple targets)." >&2
  exit 1
fi

cd "$ROOT"
for target in aarch64-apple-darwin x86_64-apple-darwin; do
  cargo build --release --locked --target "$target"
done

rm -rf "${BUNDLE:?}"
mkdir -p "$BUNDLE/server"
lipo -create -output "$DIST/angaria-mcp" \
  "target/aarch64-apple-darwin/release/angaria-mcp" \
  "target/x86_64-apple-darwin/release/angaria-mcp"
lipo -info "$DIST/angaria-mcp"

if [[ ${SKIP_PACK:-} == 1 ]]; then
  exit 0
fi

cp "$DIST/angaria-mcp" "$BUNDLE/server/angaria-mcp"
cp manifest.json LICENSE "$BUNDLE/"
npx --yes "@anthropic-ai/mcpb@$MCPB_VERSION" validate "$BUNDLE/manifest.json"
npx --yes "@anthropic-ai/mcpb@$MCPB_VERSION" pack "$BUNDLE" "$DIST/angaria-mcp.mcpb"
echo "fileSha256 for server.json:"
shasum -a 256 "$DIST/angaria-mcp.mcpb" | cut -d' ' -f1
