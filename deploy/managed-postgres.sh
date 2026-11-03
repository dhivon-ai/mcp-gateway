#!/bin/bash
# Publish a fully initialized cluster by rename. Only our unpublished stage is disposable.
set -eu
umask 077
root=/var/lib/postgresql
auth=/private/password
refuse() { echo "PostgreSQL setup refused: $*. Restore the matching postgres directory and database volume before retrying." >&2; exit 1; }
[[ "$SETUP_UID" =~ ^[1-9][0-9]*$ && "$SETUP_GID" =~ ^[0-9]+$ ]] || refuse "invalid operator identity"
owner="$SETUP_UID:$SETUP_GID"
as_owner() { gosu "$owner" "$@"; }
private_file() { [[ -f $1 && ! -L $1 && $(as_owner stat -c %u:%a "$1") = "$SETUP_UID:600" ]]; }
[[ $(as_owner stat -c %u:%a /private) = "$SETUP_UID:700" ]] || refuse "postgres directory must be mode 0700"
exec 9</private
flock -n 9 || refuse "another process owns the private directory"
# The password fingerprint also names this installation's private scratch area.
if [[ ! -e $auth && ! -L $auth ]]; then
  [[ -z $(ls -A "$root") && ! -e /private/cluster-id && ! -L /private/cluster-id ]] || refuse "existing state has lost its credentials"
  as_owner sh -ec 'umask 077; stage=$(mktemp /private/.password.XXXXXXXX); openssl rand -hex 32 > "$stage"; mv -T "$stage" /private/password'
fi
private_file "$auth" && [[ $(cat "$auth") =~ ^[0-9a-f]{64}$ ]] || refuse "invalid database password file"
fingerprint=$(sha256sum < "$auth" | cut -d' ' -f1)
stage=$root/.dhivon-init-$fingerprint
if [[ -e $PGDATA || -e /private/cluster-id || -L /private/cluster-id ]]; then
  [[ -s $PGDATA/PG_VERSION && ! -L $PGDATA && ! -L $root/$PG_MAJOR ]] || refuse "missing database cluster"
else
  [[ -z $(find "$root" -mindepth 1 -maxdepth 1 ! -name "${stage##*/}" ! -name "$PG_MAJOR" -print -quit) ]] || refuse "unrecognized volume contents"
  [[ ! -L $root/$PG_MAJOR && ( ! -e $root/$PG_MAJOR || -z $(ls -A "$root/$PG_MAJOR") ) ]] || refuse "unrecognized cluster directory"
  [[ ! -L $stage ]] || refuse "invalid staging path"
  rm -rf -- "$stage"
  install -d -o postgres -g postgres -m 700 "$stage"
  # initdb reopens its pwfile. Give it a postgres-owned pipe backed by the inherited private fd.
  gosu postgres bash -o pipefail -c 'cat | initdb -D "$1" -U dhivon --pwfile=/dev/stdin --auth-local=trust --auth-host=scram-sha-256' _ "$stage" < "$auth"
  echo 'host all all all scram-sha-256' >> "$stage/pg_hba.conf"
  echo "listen_addresses = '*'" >> "$stage/postgresql.conf"
  gosu postgres pg_ctl -D "$stage" -o "-c listen_addresses=''" -w start
  gosu postgres createdb -U dhivon dhivon
  gosu postgres pg_ctl -D "$stage" -m fast -w stop
  sha256sum < "$auth" > "$stage/.dhivon-password-sha256"
  install -d -o postgres -g postgres -m 700 "$(dirname "$PGDATA")"
  mv -T "$stage" "$PGDATA"
fi
cluster=$(LC_ALL=C pg_controldata "$PGDATA" | awk '/Database system identifier:/ {print $NF}')
[[ "$cluster" =~ ^[0-9]+$ ]] || refuse "invalid database system identifier"
# The fingerprint also authenticates a cluster published just before a crash.
[[ -f "$PGDATA/.dhivon-password-sha256" ]] || refuse "database belongs to another installation"
[[ $(sha256sum < "$auth") = "$(cat "$PGDATA/.dhivon-password-sha256")" ]] || refuse "database credential mismatch"
if [[ -e /private/cluster-id || -L /private/cluster-id ]]; then
  private_file /private/cluster-id || refuse "invalid cluster receipt"
  [[ $(cat /private/cluster-id) = "$cluster" ]] || refuse "cluster identifier mismatch"
else
  printf '%s\n' "$cluster" | gosu "$SETUP_UID:$SETUP_GID" sh -ec 'umask 077; stage=$(mktemp /private/.cluster.XXXXXXXX); cat > "$stage"; mv -T "$stage" /private/cluster-id'
fi
exec gosu postgres postgres -D "$PGDATA"
