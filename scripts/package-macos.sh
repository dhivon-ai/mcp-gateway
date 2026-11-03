#!/usr/bin/env bash
set -euo pipefail
[[ "$(uname -s)" == Darwin && "$(uname -m)" == arm64 ]] || { echo "Build on an Apple Silicon Mac" >&2; exit 1; }
cd "$(dirname "$0")/.."
# Serialize package builds from this checkout through the copy of the linked CLI.
# macOS lockf holds the directory descriptor until this script exits.
exec 9<.
lockf 9
if [[ -z "${DHIVON_MCP_GATEWAY_BUILD_CACHE:-}" ]]; then
  # Vite empties its output directory. Give each default build a private directory.
  [[ ! -L dist ]] || { echo "dist must not be a symlink" >&2; exit 1; }
  mkdir -p -m 700 dist
  [[ -O dist ]] && (( (8#$(stat -f '%Lp' dist) & 0022) == 0 )) || {
    echo "dist must be owned by you and writable only by you" >&2; exit 1;
  }
  DHIVON_MCP_GATEWAY_BUILD_CACHE=$(mktemp -d "$(pwd -P)/dist/macos-XXXXXXXX")
  if [[ -z "${CARGO_TARGET_DIR:-}" ]]; then
    CARGO_TARGET_DIR="$(pwd -P)/dist/.macos-cargo"
    # Do not adopt a pre-existing directory of unrelated files.
    if [[ ! -e "$CARGO_TARGET_DIR" && ! -L "$CARGO_TARGET_DIR" ]]; then
      mkdir -m 700 "$DHIVON_MCP_GATEWAY_BUILD_CACHE/cargo"
      touch "$DHIVON_MCP_GATEWAY_BUILD_CACHE/cargo/.package-macos"
      mv "$DHIVON_MCP_GATEWAY_BUILD_CACHE/cargo" "$CARGO_TARGET_DIR"
    fi
    marker="$CARGO_TARGET_DIR/.package-macos"
    [[ ! -L "$CARGO_TARGET_DIR" && -d "$CARGO_TARGET_DIR" && -O "$CARGO_TARGET_DIR" &&
       $(stat -f '%Lp' "$CARGO_TARGET_DIR") == 700 && -f "$marker" && ! -L "$marker" ]] || {
      echo "Expected a private Cargo cache created by this script at $CARGO_TARGET_DIR" >&2; exit 1;
    }
  fi
else
  case "$DHIVON_MCP_GATEWAY_BUILD_CACHE" in /*) ;; *) echo "Build directory must be absolute" >&2; exit 1;; esac
  mkdir -p "$DHIVON_MCP_GATEWAY_BUILD_CACHE"
  DHIVON_MCP_GATEWAY_BUILD_CACHE=$(cd "$DHIVON_MCP_GATEWAY_BUILD_CACHE" && pwd -P)
  [[ "$DHIVON_MCP_GATEWAY_BUILD_CACHE" != / ]] || { echo "Build directory cannot be the filesystem root" >&2; exit 1; }
fi
export DHIVON_MCP_GATEWAY_BUILD_CACHE
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$DHIVON_MCP_GATEWAY_BUILD_CACHE/cargo}"
export DHIVON_MCP_GATEWAY_WEB_ASSETS="$DHIVON_MCP_GATEWAY_BUILD_CACHE/web"
export MACOSX_DEPLOYMENT_TARGET=27.0.1
pnpm install --frozen-lockfile
pnpm build
cargo build --locked --release -p dhivon-mcp-gateway-cli --target aarch64-apple-darwin
version=$(sed -n 's/^version = "\([0-9.]*\)"/\1/p' Cargo.toml | head -n 1)
[[ -n "$version" ]]
name="dhivon-mcp-gateway-${version}-macos-arm64"
mkdir -p "$DHIVON_MCP_GATEWAY_BUILD_CACHE/releases"
staging=$(mktemp -d "$DHIVON_MCP_GATEWAY_BUILD_CACHE/package-XXXXXXXX")
mkdir "$staging/$name"
cp "$CARGO_TARGET_DIR/aarch64-apple-darwin/release/dhivon-mcp-gateway" "$staging/$name/dhivon-mcp-gateway"
node scripts/package-guides.ts "$staging/$name"
cp LICENSE NOTICE "$staging/$name/"
mkdir "$staging/$name/licenses"
cp assets/brand/fonts/OFL.txt "$staging/$name/licenses/Manrope-OFL.txt"
file "$staging/$name/dhivon-mcp-gateway" | grep -q 'Mach-O 64-bit executable arm64'
"$staging/$name/dhivon-mcp-gateway" --version
COPYFILE_DISABLE=1 tar -czf "$DHIVON_MCP_GATEWAY_BUILD_CACHE/releases/$name.tar.gz" -C "$staging" "$name"
node scripts/check-notices.ts --archive "$DHIVON_MCP_GATEWAY_BUILD_CACHE/releases/$name.tar.gz"
(cd "$DHIVON_MCP_GATEWAY_BUILD_CACHE/releases" && shasum -a 256 "$name.tar.gz" > "$name.tar.gz.sha256" && shasum -a 256 -c "$name.tar.gz.sha256")
# Only the mktemp directory created by this invocation is removed.
rm -r "$staging"
printf 'Archive: %s/releases/%s.tar.gz\n' "$DHIVON_MCP_GATEWAY_BUILD_CACHE" "$name"
printf 'Checksum: %s/releases/%s.tar.gz.sha256\n' "$DHIVON_MCP_GATEWAY_BUILD_CACHE" "$name"
