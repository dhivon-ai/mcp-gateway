#!/usr/bin/env bash
# Export from the root Compose project and publish only a completed archive.
set -eu
umask 077
fail() { printf '%s\n' "$*" >&2; exit 1; }
[[ $# = 1 && $1 = /* ]] || fail 'Supply an absolute path to a private backup directory outside the checkout.'
[[ $1 != *$'\n'* && $1 != *$'\r'* ]] || fail 'Use a backup path without line breaks.'
root=$(cd "$(dirname "$0")/.." && pwd -P)
parent=$(cd -P "$(dirname "$1")" && pwd -P) || fail 'Create the parent of the backup directory first.'
name=$(basename "$1")
[[ $name != . && $name != .. && $name != / ]] || fail 'Supply the named backup directory.'
directory=${parent%/}/$name
# Compare directory identities so alternate casing cannot bypass the checkout boundary.
ancestor=$directory
while [[ -n $ancestor ]]; do
  [[ ! $ancestor -ef $root ]] || fail 'Choose a backup directory outside the checkout.'
  ancestor=${ancestor%/*}
done
mkdir -p -m 700 "$directory"
[[ -d $directory && ! -L $directory && -O $directory ]] || fail 'Use a private backup directory you own.'
cd -P "$directory"
if [[ $(uname -s) = Darwin ]]; then mode=$(stat -f %Lp .); else mode=$(stat -c %a .); fi
[[ $mode = 700 ]] || fail 'Set the backup directory to mode 0700.'
stage=$(mktemp .database.XXXXXXXX)
trap 'rm -f -- "$stage"' EXIT
dump_pid=
interrupt() {
  if [[ -n $dump_pid ]]; then kill "$dump_pid" 2>/dev/null || true; fi
  exit 1
}
trap interrupt HUP INT TERM
(cd "$root" && exec docker compose exec -T postgres pg_dump -U dhivon -d dhivon -Fc) > "$stage" &
dump_pid=$!
wait "$dump_pid"
dump_pid=
[[ -s $stage ]] || fail 'PostgreSQL produced an empty archive.'
archive="database-$(date -u +%Y%m%dT%H%M%SZ)-${stage##*.}.dump"
link "$stage" "$archive"
printf '%s/%s\n' "$(pwd -P)" "$archive"
