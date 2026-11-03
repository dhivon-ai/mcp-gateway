# Review Bugs: consumer-website @ 2026-11-03

- Review cycle: 1
- Disposition: all seven block release; priority describes impact.
- Reviewed commit range: `cd7253b598256b1d02cc9545c2cb5ce08a3632e7..a1b88eb7bb4213806fdf869510e5a92f0eaee526`
- Human approval: bound to the review operation receipt on promotion.

## BUG-001: Linux database initialization fails for a distinct deployment UID

- Severity: Release blocker (P1)
- Evidence: deploy/managed-postgres.sh:31
- Reproduction: Run the shipped wrapper in the pinned Postgres image with empty Linux tmpfs mounts and /private mode 0700 owned by UID 1001. initdb running as postgres cannot reopen the owner-only password through /dev/stdin (Permission denied).
- Impact: A normal Linux operator cannot bring up a fresh gateway. AC-003, NFR-002.
- Remediation acceptance criteria: Start with distinct UIDs on a native Linux filesystem; reach healthy PostgreSQL and first-admin sign-in, restart with unchanged credentials. Keep the operator file 0600 and secrets out of arguments and logs.

## BUG-002: Server help discloses the database credential

- Severity: Release blocker (P2)
- Evidence: deploy/managed-gateway.sh:12; crates/dhivon-mcp-gateway-server/src/main.rs:21
- Reproduction: Invoke the built server with --help and a synthetic password in DATABASE_URL as provided by the wrapper. The full URL, including the sentinel, appeared in stdout.
- Impact: An operator copying help into support output can disclose the database password. No actual credential leak was observed. NFR-002.
- Remediation acceptance criteria: Keep the option usable; check top-level help, subcommand help and invalid invocation stdout/stderr with a sentinel. Neither password nor full URI appears.

## BUG-003: Interrupted configuration prevents a normal retry

- Severity: Release blocker (P2)
- Evidence: scripts/configure-gateway.sh:40–45
- Reproduction: In a private checkout fixture, force mv to fail after its first move. No .env is published; rerunning the unmodified helper exits 1 with Incomplete private setup.
- Impact: A first-time operator needs manual recovery before any service is deployed. AC-003.
- Remediation acceptance criteria: Interrupt before and during publication, then rerun in the same location. Publish one complete private setup, preserve committed inputs, refuse foreign state and handle competing runs without overwriting another setup.

## BUG-004: Setup saves inputs that provisioning cannot use

- Severity: Release blocker (P2)
- Evidence: scripts/configure-gateway.sh:22–25; crates/dhivon-mcp-gateway-server/src/identity.rs:65; crates/dhivon-mcp-gateway-core/src/policy.rs:241
- Reproduction: Run the real helper separately with a..b@example.com and an HTTPS origin on port 99999. Both return success and write .env. Server email rules reject adjacent dots and the URL parser rejects out-of-range ports.
- Impact: A typo locks in unusable settings; rerunning refuses the existing .env. FR-004, AC-003.
- Remediation acceptance criteria: Reject invalid inputs before publishing deployment files, permit correction, and exercise a valid email and HTTPS origin with an explicit valid port through provisioning.

## BUG-005: Backup instructions can destroy a previous dump

- Severity: Release blocker (P2)
- Evidence: docs/operations.md:9
- Reproduction: The documented shell redirection opens database.dump in the checkout before pg_dump runs. Substituting a failing producer truncated a fixture previous backup to zero bytes. git check-ignore confirms this path is not ignored.
- Impact: A stopped stack or an interrupted dump can destroy the prior backup or leave a partial file; an operator could accidentally commit private data. AC-003, NFR-002/003.
- Remediation acceptance criteria: Write owner-only temporary output outside the checkout and publish a distinct usable backup only after success. Rehearse immediate and partial-output failure, preservation of the prior dump, successful restore and saved-credential read with separately held vault keys.

## BUG-006: Every default Mac build retains another Cargo target

- Severity: Release blocker (P2)
- Evidence: scripts/package-macos.sh:9,18
- Reproduction: The default creates a new private directory each time and puts CARGO_TARGET_DIR under it. One actual default Apple Silicon build retained 748 MiB (du -sh: 748M) in cargo alone.
- Impact: Repeat builds on one workstation recompile dependencies and consume disk even when source has not changed. This is a repeat-build cost, not a first-build failure. FR-005.
- Remediation acceptance criteria: Run twice with defaults: reuse one safe Cargo target without accumulating another full target. Preserve per-build release outputs, current embedded assets, unrelated user files, and explicit build/cache overrides.

## BUG-007: Archive installation guide links to missing files

- Severity: Release blocker (P3)
- Evidence: docs/mac-install.md:22,34; scripts/package-macos.sh:31–32
- Reproduction: Compare README links with the final tar listing: neither endpoint-identity.md nor agent-configuration.md is present. The website crawl cannot exercise these archive-relative links.
- Impact: Someone with only the archive cannot follow the upgrade or connect-your-agent link. AC-004/005, NFR-003.
- Remediation acceptance criteria: Follow every README/compatibility link and anchor from a freshly extracted archive outside the checkout; bundle the targets or point to verified public guides. Check the release checksum, notices, version and first agent connection.
