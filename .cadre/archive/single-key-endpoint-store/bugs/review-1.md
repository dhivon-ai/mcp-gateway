# Review Bugs: single-key-endpoint-store @ 2026-11-03T04:04:07+00:00

- Review cycle: 1
- Status: proposed; pending the owner's review decision
- Reviewed commit range: `1d5aa125ee6b7643192f3dd5fe8bf58e2ef0dbff..43bae33639e41bd0154486fc3436df6945c092be`
- Scope: fresh installations of the current format. The owner waived backward compatibility; no legacy upgrade work is proposed.

## BUG-001: A known refusal can be lost when the credential and its recovery evidence are unreadable

- Severity: P1 (authorization).
- Evidence: `crates/dhivon-mcp-gateway-cli/src/upstream_credentials.rs:504`. In `reject`, failure to read the fallback evidence returns before recording the server's refusal. The caller denies that one request, but the previously selected record IDs remain authoritative.
- Impact: Within the existing lease the gateway can supply a static key that the control plane has already refused. No master-key access or forgery is required to restore the two ciphertexts.
- Reproduction: A native Mac fixture established a real Keychain-backed current-format store and used the actual resolver against authenticated HTTP. The first response (200) issued a synthetic bearer. Save and damage only the two sealed files holding its head and recovery evidence; a 403 for that same MCP is denied immediately. Restore only those two files; on the third response (503), the resolver supplied a usable lease with the original binding. The existing one-record repair and ordinary rejection tests passed. No external provider call was made.
- Remediation acceptance criteria: Record a durable denial for the still-current scope even when the old credential is unreadable. Restore the saved files and restart a reader during an outage: it must not obtain the old bearer. Only a valid subsequent online authorization may restore eligibility, with existing version, rejection and expiry rules. Preserve unrelated MCPs and fence stale responses after identity/binding changes. FR-003, FR-004, AC-002 and AC-003.
- Human approval: Pending digest-bound decision.

## BUG-002: Loss of the state directory can prevent the current console from offering sign-in

- Severity: P2 (recovery and availability).
- Evidence: `crates/dhivon-mcp-gateway-cli/src/cutover.rs:166` permits recovery from an unreadable index only with an exact on-disk format marker. Line 66 also treats an oversized marker as fatal before inspecting healthy encrypted state.
- Impact: Users who still have the master Keychain item but lose the local directory cannot reach the independent authentication flow. Damage to a nonsecret marker can also block an otherwise healthy installation.
- Reproduction: Initialize a fresh directory with the packaged v2 Mac CLI; the UI prints a loopback URL. With that same master present, replace the local directory with an empty private directory. The same UI command exits 1 advising online sync before opening the console. Separately, preserve every encrypted file and overwrite just the marker with 65,537 bytes: both status and UI exit 1. Restoring the original fixture permitted verified retirement. Neither case used a v1 binary or data.
- Remediation acceptance criteria: Start the console with an explicit server and offer independent sign-in after local directory loss. A failed authentication cannot replace the master; a successful one rebuilds, and a restored old directory cannot grant access. A damaged marker must not veto authenticated healthy state. Continue rejecting unsafe paths and inaccessible Keychains; never infer authentication from an ordinary file. FR-001, FR-004 and AC-003.
- Human approval: Pending digest-bound decision.
