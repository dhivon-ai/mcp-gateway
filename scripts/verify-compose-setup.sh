#!/usr/bin/env bash
# Exercise only a new, private root Compose project with a disposable database.
set -eu
umask 077
root=$(cd "$(dirname "$0")/.." && pwd -P)
cache=$(cd "${DHIVON_MCP_GATEWAY_BUILD_CACHE:?Set the owned build cache directory}" && pwd -P)
dir=$(mktemp -d "$cache/compose-setup.XXXXXXXX")
project="dhivon-setup-$(openssl rand -hex 12)"
export DHIVON_MCP_GATEWAY_PUBLIC_URL=https://gateway.example.test
export DHIVON_MCP_GATEWAY_SETUP_DIR=$dir DHIVON_MCP_GATEWAY_PORT=0
export DHIVON_MCP_GATEWAY_UID="$(id -u)" DHIVON_MCP_GATEWAY_GID="$(id -g)"
compose() { docker compose --env-file /dev/null -p "$project" -f "$root/compose.yaml" "$@"; }
cleanup() {
  result=$?
  trap - EXIT
  compose down --volumes || exit 1
  for service in setup dhivon; do
    if docker image inspect "$project-$service" >/dev/null 2>&1; then
      docker image rm "$project-$service" >/dev/null || exit 1
    fi
  done
  if [[ $result = 0 ]]; then rm -rf -- "$dir"; else echo "Private verification directory retained: $dir" >&2; fi
  exit "$result"
}
trap cleanup EXIT
mkdir -m 700 "$dir/gateway" "$dir/postgres"
printf '%s\n' admin@example.test > "$dir/admin-email"
openssl rand -hex 32 > "$dir/admin-password"
compose up -d --build --wait
key="$dir/gateway/keys/vault-keys.json"
identity=$(openssl dgst -sha256 "$key" "$dir/postgres/password" "$dir/gateway/keys/deployment-id")
[[ ! -e $dir/admin-password ]]
compose up -d --build --wait
[[ $(openssl dgst -sha256 "$key" "$dir/postgres/password" "$dir/gateway/keys/deployment-id") = "$identity" ]]
[[ $(compose exec -T postgres psql -U dhivon -d dhivon -Atc "SELECT count(*) FROM users WHERE role='admin'") = 1 ]]
[[ -z $(docker port "$(compose ps -q postgres)") ]]

# A fresh key on an existing database must stop startup before it serves traffic.
compose stop dhivon
compose run --rm --no-deps -T --entrypoint dhivon-mcp-gateway-server setup \
  generate-vault-key --output /setup/wrong-key
mv "$key" "$dir/matching-key"
mv "$dir/wrong-key" "$key"
if compose up -d --wait; then echo 'Wrong key unexpectedly accepted.' >&2; exit 1; fi
[[ -z $(compose ps -q --status running dhivon) ]]
mv "$dir/matching-key" "$key"
compose up -d --wait
echo 'PASS: managed first admin, private database, stable restart, wrong-key refusal and recovery.'
