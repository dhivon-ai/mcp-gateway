#!/usr/bin/env bash
# Prepare private inputs for the root Compose file. Run as the deployment owner.
set -eu
umask 077
root=$(cd "$(dirname "$0")/.." && pwd -P)
target=$root/.env
fail() { printf '%s\n' "$*" >&2; exit 1; }
mode() {
  if [[ $(uname -s) = Darwin ]]; then stat -f %Lp "$1"; else stat -c %a "$1"; fi
}
[[ $(id -u) != 0 ]] || fail 'Run as the unprivileged deployment owner.'
[[ ! -e $target && ! -L $target ]] || fail 'Keeping the existing .env. To start its deployment, run docker compose up -d --build --wait.'
config=${XDG_CONFIG_HOME:-"$HOME/.config"}
[[ $config = /* ]] || fail 'Use an absolute XDG_CONFIG_HOME outside the checkout.'
mkdir -p -- "$config"
config=$(cd "$config" && pwd -P)
directory=$config/dhivon-mcp-gateway
case "$directory" in "$root"|"$root"/*|*"'"*|*\\*|*$'\n'*|*$'\r'*) fail 'Choose a private config location outside this checkout with a plain path.';; esac
if [[ ! -e $directory && ! -L $directory ]]; then
  printf 'Public HTTPS origin (for example https://gateway.example.com): ' >&2
  IFS= read -r origin
  [[ $origin =~ ^https://[a-zA-Z0-9.-]+(:[0-9]+)?$ ]] || fail 'Enter an HTTPS hostname and optional port, without a path.'
  printf 'First administrator email: ' >&2
  IFS= read -r email
  [[ $email = *@* && $email != *[[:space:]]* ]] || fail 'Enter a valid administrator email.'
  printf 'First administrator password (12–128 bytes, hidden): ' >&2
  IFS= read -r -s password
  printf '\n' >&2
  LC_ALL=C
  (( ${#password} >= 12 && ${#password} <= 128 )) || fail 'Password must contain 12–128 bytes.'
  stage=$(mktemp -d "$config/.dhivon-setup.XXXXXXXX")
  trap 'rm -rf -- "$stage"' EXIT
  mkdir -m 700 "$stage/postgres" "$stage/gateway"
  printf '%s\n' "$email" > "$stage/admin-email"
  printf '%s' "$password" > "$stage/admin-password"
  unset password
  printf "DHIVON_MCP_GATEWAY_PUBLIC_URL='%s'\nDHIVON_MCP_GATEWAY_SETUP_DIR='%s'\nDHIVON_MCP_GATEWAY_UID=%s\nDHIVON_MCP_GATEWAY_GID=%s\n" \
    "$origin" "$directory" "$(id -u)" "$(id -g)" > "$stage/compose.env"
  # Claim the destination without replacing another operator's configuration.
  mkdir -m 700 "$directory" || fail 'Another setup claimed the private directory.'
  mv "$stage"/{admin-email,admin-password,postgres,gateway,compose.env} "$directory/"
fi
[[ -d $directory && ! -L $directory && -O $directory && $(mode "$directory") = 700 &&
   -f $directory/compose.env && ! -L $directory/compose.env && -O $directory/compose.env &&
   $(mode "$directory/compose.env") = 600 ]] || fail 'Incomplete private setup. Recover the existing directory or choose a fresh XDG_CONFIG_HOME.'
# Publish complete settings without overwriting an existing checkout .env.
settings=$(mktemp "$root/.env.configure.XXXXXXXX")
trap 'rm -f -- "$settings"; if [[ -n ${stage:-} ]]; then rm -rf -- "$stage"; fi' EXIT
cat "$directory/compose.env" > "$settings"
ln "$settings" "$target" || fail 'Keeping the existing checkout .env.'
printf 'Private deployment directory: %s\nRun from the repository root: docker compose up -d --build --wait\n' "$directory"
