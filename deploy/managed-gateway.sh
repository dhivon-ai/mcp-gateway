#!/bin/sh
set -eu
umask 077
test "$(id -u)" != 0 || { echo 'Use a nonroot gateway UID.' >&2; exit 1; }
db_file=/setup/postgres/password
if [ -L "$db_file" ] || [ ! -f "$db_file" ] || [ "$(stat -c %u:%a "$db_file")" != "$(id -u):600" ]; then
  echo 'Restore the matching owner-only database password file.' >&2
  exit 1
fi
password=$(cat "$db_file")
case "$password" in ''|*[!0-9a-f]*) echo 'Restore the matching database password file.' >&2; exit 1;; esac
export DATABASE_URL="postgres://dhivon:$password@postgres:5432/dhivon"
unset password
[ "$#" -gt 0 ] || set -- serve-managed
exec dhivon-mcp-gateway-server "$@"
