#!/bin/sh
set -eu
authority=${DHIVON_MCP_GATEWAY_PUBLIC_URL#*://}
authority=${authority%/}
case "$DHIVON_MCP_GATEWAY_PUBLIC_URL" in
  https://*) authority=${authority%:443} ;;
  http://*) authority=${authority%:80} ;;
esac
exec curl --fail --silent --show-error --max-time 5 --noproxy '*' \
  -H "Host: $authority" http://127.0.0.1:8080/healthz >/dev/null
