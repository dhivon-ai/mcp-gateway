#!/usr/bin/env bash
# Run the documented backup entry point in a disposable checkout with a controllable pg_dump.
set -eu
umask 077
source_root=$(cd "$(dirname "$0")/.." && pwd -P)
cache=$(cd "${DHIVON_MCP_GATEWAY_BUILD_CACHE:?Set the owned build cache directory}" && pwd -P)
fixture=$(mktemp -d "$cache/verify-backup.XXXXXXXX")
trap 'rm -rf -- "$fixture"' EXIT
export checkout="$fixture/checkout"
mkdir -p "$checkout/scripts" "$fixture/bin"
cp "$source_root/scripts/backup-gateway.sh" "$checkout/scripts/"
cd "$checkout"
export real_link=$(command -v link)
cat > "$fixture/bin/docker" <<'SH'
#!/bin/sh
set -eu
test "$PWD" = "$checkout"
test "$*" = 'compose exec -T postgres pg_dump -U dhivon -d dhivon -Fc'
case "$probe" in
  immediate) exit 12;;
  partial) printf 'partial output'; exit 13;;
  empty) exit 0;;
  interrupt) printf 'partial output'; touch "$signal_ready"; exec sleep 30;;
  *) printf '%s' "$probe";;
esac
SH
cat > "$fixture/bin/link" <<'SH'
#!/bin/sh
set -eu
if [ "$probe" = collision ]; then printf 'existing dump' > "$2"; fi
exec "$real_link" "$@"
SH
chmod 700 "$fixture/bin/"*
export PATH="$fixture/bin:$PATH"
backups="$fixture/private backups"
backup() { bash scripts/backup-gateway.sh "$@"; }
refused() {
  if backup "$@" > "$fixture/stdout"; then echo 'Unexpected backup success.' >&2; exit 1; fi
  test ! -s "$fixture/stdout"
}
probe=first; export probe
first=$(backup "$backups")
test "$(dirname "$first")" = "$backups"
test "$(cat "$first")" = first
test "$(find "$backups" -type d -perm 0700)" = "$backups"
test "$(find "$backups" -type f -perm 0600)" = "$first"
for probe in immediate partial empty; do
  refused "$backups"
  test "$(find "$backups" -type f)" = "$first"
done

export signal_ready="$fixture/signal-ready"
probe=interrupt
bash scripts/backup-gateway.sh "$backups" > "$fixture/stdout" &
pid=$!
for _ in {1..100}; do [[ -e $signal_ready ]] && break; sleep 0.05; done
test -e "$signal_ready"
kill -TERM "$pid"
if wait "$pid"; then echo 'Interrupted backup succeeded.' >&2; exit 1; fi
test ! -s "$fixture/stdout"
test "$(find "$backups" -type f)" = "$first"

probe=second
second=$(backup "$backups")
test "$first" != "$second"
test "$(cat "$second")" = second
probe=collision
refused "$backups"
other=$(find "$backups" -type f ! -path "$first" ! -path "$second")
test "$(cat "$other")" = 'existing dump'
test "$(cat "$first")" = first

mkdir -m 755 "$fixture/shared"
ln -s "$checkout" "$fixture/checkout-alias"
for bad in relative "$checkout" "$checkout/private" "$fixture/checkout-alias/private" "$fixture/shared"; do
  refused "$bad"
done
if [[ "$fixture/CHECKOUT" -ef $checkout ]]; then
  refused "$fixture/CHECKOUT/scripts/private"
fi
test "$(find "$checkout" -mindepth 1 | sort)" = "$(printf '%s\n' "$checkout/scripts" "$checkout/scripts/backup-gateway.sh")"
echo 'PASS: backup failures, interruption, repeated success, exclusive publication and private off-checkout output.'
