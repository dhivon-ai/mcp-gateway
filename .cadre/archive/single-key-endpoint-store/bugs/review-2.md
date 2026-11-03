# Review Bugs: single-key-endpoint-store

- Review cycle: 2
- Status: proposed for approval
- Reviewed range: `d29660c1249ef046f62af09baf6263646c4c6608..519aa1973ced5137070d630c995cd446ee0504c0`
- Reviewed scope: fresh installations using the current Mac format. Both original reproduction tests pass at the reviewed head.

## BUG-001 follow-up: An in-flight first fetch can override a newer refusal

- Severity: P1, authorization.
- Evidence: `crates/dhivon-mcp-gateway-cli/src/upstream_credentials.rs:515` records an independent refusal stamp only when an account is already selected. The fallback at line 589 publishes a negative head without a stamp. The comparison at line 386 sees the same absent stamp as the earlier request.
- Reproduction: Use a fresh native Mac fixture with a signed-in session, valid policy and no cached head for the assigned static key. Hold the first authenticated HTTP 200 response open. Allow a second request to receive HTTP 403 and finish. Release the first response, then return HTTP 503 to a third request. The second request was denied, but the first and third both obtained the bearer. The probe exercised the production resolver and real Keychain-backed store, then retired its fixture.
- Impact: A credential can first become available after its authorization has already been refused and remain usable during an outage.
- Acceptance: The first refusal establishes a durable decision for the current generation, MCP and binding. A response started before that decision cannot publish. Test the exact HTTP sequence, offline restart, a later valid online grant and isolation from other bindings. Preserve expiry and provider signature rules. FR-003, AC-002, T5.1.

## BUG-002 follow-up: Truncating the marker and index together prevents sign-in

- Severity: P2, recovery.
- Evidence: `crates/dhivon-mcp-gateway-cli/src/cutover.rs:77` classifies a present empty marker differently from an absent marker. When the encrypted index is unreadable, line 177 only allows recovery with a matching or absent marker.
- Reproduction: Initialize the released arm64 CLI in a new private fixture directory. Keep its master in Keychain and truncate the ordinary format marker and the selected encrypted index to zero bytes. Run the packaged UI with an explicit loopback fixture server. It exited 1 before offering sign-in and advised online synchronization. Returning the saved fixture bytes permitted normal retirement.
- Impact: Following partial filesystem damage the console cannot perform the independent authentication needed to rebuild.
- Acceptance: With a present master and unreadable index, an empty, malformed or oversized private format marker must permit independent sign-in for current-format recovery. Verify failed authentication preserves the master, successful authentication rebuilds and rejects replay, and unsafe path or Keychain access errors remain errors. FR-004, AC-003, T5.2.

Approval for the proposed remediation is pending.
