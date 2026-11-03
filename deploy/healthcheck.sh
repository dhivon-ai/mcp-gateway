#!/bin/sh
set -eu
origin=$(dhivon-mcp-gateway-server --allow-loopback-http public-origin)
authority=${origin#*://}
exec curl --fail --silent --show-error --max-time 5 --noproxy '*' \
  -H "Host: $authority" http://127.0.0.1:8080/healthz >/dev/null
