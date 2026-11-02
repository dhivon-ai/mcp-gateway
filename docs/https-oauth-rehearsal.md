# Docker HTTPS and native OAuth rehearsal

This test simulates a hosted control plane using the real service image,
PostgreSQL and an isolated nginx TLS proxy. The extracted Mac client connects
over verified HTTPS. A controlled MCP/OAuth fixture is exposed through a second
HTTPS listener. The browser returns to the Mac's actual
`http://127.0.0.1:{port}/oauth/callback` listener.

The fixture has a preregistered public native client,
`dhivon-mcp-gateway-fixture-client`, and supports the gateway's discovery profile,
S256 PKCE, state/issuer validation, resource binding and rotating refresh tokens.
It does not authenticate a real person and must never be publicly exposed.
The proxy publishes only loopback ports. No DNS or public hosting is claimed.

## Prerequisites

Use the pinned Node/Rust toolchains, Docker on the validated Apple Silicon Mac,
an owned `DHIVON_MCP_GATEWAY_BUILD_CACHE`, an extracted current archive and a
current fixture binary. Build them with the [release checks](testing.md).

```sh
export DHIVON_MCP_GATEWAY_TEST_CLI=/absolute/path/to/extracted/dhivon-mcp-gateway
export DHIVON_MCP_GATEWAY_FIXTURE_BIN="$CARGO_TARGET_DIR/debug/dhivon-mcp-gateway-fixture"
```

The script accepts no deployment URL, production database, account or Compose
project. It generates private fixture resources. It retains the creating Mac
executable until logout confirms empty identity and Keychain cleanup metadata.

## Create temporary local certificates

Generate a fresh private directory; never overwrite a certificate already in use.

```sh
umask 077
export DHIVON_MCP_GATEWAY_TEST_TLS_DIR=$(mktemp -d "$DHIVON_MCP_GATEWAY_BUILD_CACHE/tls-XXXXXXXX")
openssl req -x509 -newkey rsa:2048 -sha256 -days 1 -nodes \
  -subj '/CN=Dhivon isolated HTTPS simulation' \
  -addext 'basicConstraints=critical,CA:TRUE' \
  -keyout "$DHIVON_MCP_GATEWAY_TEST_TLS_DIR/ca.key" \
  -out "$DHIVON_MCP_GATEWAY_TEST_TLS_DIR/ca.pem"
openssl req -newkey rsa:2048 -nodes -subj '/CN=localhost' \
  -keyout "$DHIVON_MCP_GATEWAY_TEST_TLS_DIR/server.key" \
  -out "$DHIVON_MCP_GATEWAY_TEST_TLS_DIR/server.csr"
cat > "$DHIVON_MCP_GATEWAY_TEST_TLS_DIR/server.ext" <<'EOF'
subjectAltName=DNS:localhost,IP:127.0.0.1
basicConstraints=critical,CA:FALSE
keyUsage=critical,digitalSignature,keyEncipherment
extendedKeyUsage=serverAuth
EOF
openssl x509 -req -sha256 -days 1 \
  -in "$DHIVON_MCP_GATEWAY_TEST_TLS_DIR/server.csr" \
  -CA "$DHIVON_MCP_GATEWAY_TEST_TLS_DIR/ca.pem" \
  -CAkey "$DHIVON_MCP_GATEWAY_TEST_TLS_DIR/ca.key" -CAcreateserial \
  -extfile "$DHIVON_MCP_GATEWAY_TEST_TLS_DIR/server.ext" \
  -out "$DHIVON_MCP_GATEWAY_TEST_TLS_DIR/server.pem"
openssl x509 -in "$DHIVON_MCP_GATEWAY_TEST_TLS_DIR/ca.pem" -noout -fingerprint -sha256
```

The unchanged native client uses macOS certificate trust. An operator must approve
the displayed CA fingerprint and temporary user-level SSL trust for **localhost
only** before the following command. The test script does not grant trust itself.
macOS may require the operator to authorize the trust change. Keep certificate
and hostname verification enabled; do not use an insecure client or bypass a
browser certificate warning.

```sh
security add-trusted-cert -r trustRoot -p ssl -s localhost \
  -k "$HOME/Library/Keychains/login.keychain-db" \
  "$DHIVON_MCP_GATEWAY_TEST_TLS_DIR/ca.pem"
```

## Run and complete the browser redirect

```sh
node scripts/verify-https-oauth.ts
```

The script starts generated Compose resources, bootstraps an isolated account,
configures the MCP and starts the extracted Mac UI. It verifies HTTPS health,
native sign-in/sync, the exact local redirect URI, S256 PKCE and rejection of
forged state/issuer. It prints a private `browser.json` path. Open its
`authorizationUrl` in a browser on the same Mac within three minutes.

The one-day localhost CA worked with Safari's macOS trust handling in the recorded
rehearsal. In-app Chromium and Chrome rejected that constrained test CA; their
certificate warnings were not bypassed. Use a browser that accepts the approved
trust, or ask the operator to handle a warning themselves. Do not grant broader
trust merely to finish a test.

The controlled authorization endpoint redirects to the local listener. Expect
**OAuth connected**. The script then verifies that consent did not enable the
MCP, restarts management to check persistence, explicitly enables, invokes a tool
through a real Node STDIO agent, waits for access-token expiry, waits for the existing 60-second sync/reconciliation cycle to restore discovery
after expiry, verifies refresh and disconnect, and confirms that a subsequent call is denied.

Success prints PASS results and retains a redacted `result.json`. It stops owned
processes, logs out with the immutable creating binary, verifies native cleanup
and removes only its generated Compose stack/volumes. On failure retain the
private directory and creating binary until cleanup is reconciled; do not delete
state that still owns Keychain credentials. Certificate trust is a separate step.

## Remove temporary trust and certificate

After every successful or failed run, remove only the approved CA's trust and
certificate. Do this after native logout and stack cleanup. macOS can require
operator authorization to remove trust settings as well as to add them.

```sh
security remove-trusted-cert "$DHIVON_MCP_GATEWAY_TEST_TLS_DIR/ca.pem"
dhivon_test_ca_sha256=$(openssl x509 -in "$DHIVON_MCP_GATEWAY_TEST_TLS_DIR/ca.pem" -noout -fingerprint -sha256 | cut -d= -f2 | tr -d ':')
security delete-certificate -Z "$dhivon_test_ca_sha256" "$HOME/Library/Keychains/login.keychain-db"
```

Verify that this exact fingerprint is absent from the login keychain and that
`security verify-cert -c "$DHIVON_MCP_GATEWAY_TEST_TLS_DIR/server.pem" -p ssl -s localhost`
now fails. Retain the redacted result; keep private keys out of repository and
published evidence.

## Evidence limits

This scenario proves the deployed HTTPS/native-loopback architecture using
controlled services. It does not validate public DNS, a publicly trusted hosting
certificate, real-user provider consent or a named external provider's registration
policy. Those remain separate compatibility work. See the [OAuth profile](oauth.md)
and [current release evidence](dhivon-release.md).
