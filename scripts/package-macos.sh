#!/usr/bin/env bash
set -euo pipefail
: "${GATEWAY_BUILD_CACHE:?Set an absolute owned build directory}"
case "$GATEWAY_BUILD_CACHE" in /*) ;; *) echo "Build directory must be absolute" >&2; exit 1;; esac
[[ "$GATEWAY_BUILD_CACHE" != / ]]
[[ "$(uname -s)" == Darwin && "$(uname -m)" == arm64 ]] || { echo "Build on an Apple Silicon Mac" >&2; exit 1; }
cd "$(dirname "$0")/.."
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$GATEWAY_BUILD_CACHE/cargo}"
export GATEWAY_WEB_ASSETS="$GATEWAY_BUILD_CACHE/web"
export MACOSX_DEPLOYMENT_TARGET=27.0.1
pnpm install --frozen-lockfile
pnpm build
cargo build --locked --release -p gateway-cli --target aarch64-apple-darwin
version=$(sed -n 's/^version = "\([0-9.]*\)"/\1/p' Cargo.toml | head -n 1)
[[ -n "$version" ]]
name="mcp-gateway-${version}-macos-arm64"
mkdir -p "$GATEWAY_BUILD_CACHE/releases"
staging=$(mktemp -d "$GATEWAY_BUILD_CACHE/package-XXXXXXXX")
mkdir "$staging/$name"
cp "$CARGO_TARGET_DIR/aarch64-apple-darwin/release/mcp-gateway" "$staging/$name/mcp-gateway"
cp docs/mac-install.md "$staging/$name/README.md"
cp docs/compatibility.md "$staging/$name/COMPATIBILITY.md"
file "$staging/$name/mcp-gateway" | grep -q 'Mach-O 64-bit executable arm64'
"$staging/$name/mcp-gateway" --version
COPYFILE_DISABLE=1 tar -czf "$GATEWAY_BUILD_CACHE/releases/$name.tar.gz" -C "$staging" "$name"
(cd "$GATEWAY_BUILD_CACHE/releases" && shasum -a 256 "$name.tar.gz" > "$name.tar.gz.sha256" && shasum -a 256 -c "$name.tar.gz.sha256")
# Only the mktemp directory created by this invocation is removed.
rm -r "$staging"
printf 'Archive: %s/releases/%s.tar.gz\n' "$GATEWAY_BUILD_CACHE" "$name"
