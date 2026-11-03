# Consumer website review, cycle 2

- Reviewed range: `13a0a131c7e84ec1ddfd202a5e222550086259ff..be6570c62bf88048a74f30545de77088e9410904`
- Disposition: changes requested. Both findings block release under the requested policy.
- Approval is recorded by the review receipt upon promotion.

The seven original reproductions have passing implementation evidence. During this review, all eight configuration tests, the backup failure suite and the 33-file publication check passed. BUG-004 remains open for a different port spelling. BUG-001–003 and BUG-005–007 have no outstanding findings from their acceptance checks.

## BUG-004 (reopened): A padded HTTPS port fails the shipped health check

- Priority: P2, release blocker.
- Evidence: scripts/configure-gateway.py:52 writes the supplied port spelling. deploy/healthcheck.sh:3 uses it in Host. crates/dhivon-mcp-gateway-server/src/lib.rs:205 requires the authority computed by the URL parser.
- Reproduction: In a fresh private deployment the helper accepted https://gateway.example.test:08443. The running service returned 200 for Host gateway.example.test:8443 and 400 for Host gateway.example.test:08443. The shipped health check exited 22.
- Impact: The documented Compose command waits for a healthy service even though the gateway already responds to the canonical address (FR-004, AC-003).
- Acceptance: Produce consistent authorities for padded default and nondefault ports across setup, gateway and the shipped health check. Exercise canonical and padded forms, reject invalid ports before publication, and prove a fresh documented start and restart without changing credentials.

## BUG-008: Setup rejects a valid internationalized hostname

- Priority: P2, release blocker.
- Evidence: scripts/configure-gateway.py:49 applies Python's built-in IDNA decoder. The server validates the same input with the Rust URL parser before connecting to PostgreSQL in crates/dhivon-mcp-gateway-server/src/lib.rs:51.
- Reproduction: The helper's origin validator rejects xn--fa-hia.example.test, the ASCII form of faß.example.test. The built Rust server accepted that URL through validation and reached the deliberately unavailable test database. With malformed xn--a.example.test the same server stopped at URL validation.
- Impact: An operator using the valid ASCII hostname cannot complete first-time setup. This also regresses a hostname accepted by the previous setup helper (FR-004, AC-003).
- Acceptance: Support the valid ASCII hostname through the one-time setup and gateway, reject the malformed control before publishing settings, and verify a real HTTPS login through the valid hostname using a local test certificate. Keep setup prerequisites documented.
