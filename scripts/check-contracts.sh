#!/usr/bin/env bash
set -euo pipefail
: "${GATEWAY_BUILD_CACHE:?Set GATEWAY_BUILD_CACHE to the owned build directory}"
output="$GATEWAY_BUILD_CACHE/contracts-check"
mkdir -p "$output"
cargo run --locked -p gateway-core --example export_contracts -- "$output"
diff -ru apps/web/src/generated "$output"
