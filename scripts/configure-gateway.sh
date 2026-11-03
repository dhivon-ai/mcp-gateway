#!/usr/bin/env bash
# Prepare private inputs for the root Compose file. Run as the deployment owner.
set -eu
exec python3 "$(dirname "$0")/configure-gateway.py"
