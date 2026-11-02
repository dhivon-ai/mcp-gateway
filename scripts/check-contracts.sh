#!/usr/bin/env bash
set -euo pipefail
: "${DHIVON_MCP_GATEWAY_BUILD_CACHE:?Set DHIVON_MCP_GATEWAY_BUILD_CACHE to the owned build directory}"
output="$DHIVON_MCP_GATEWAY_BUILD_CACHE/contracts-check"
mkdir -p "$output"
cargo run --locked -p dhivon-mcp-gateway-core --example export_contracts -- "$output"
diff -ru apps/web/src/generated "$output"
