# Specification: Single Keychain master key for all local gateway state

- Track: `single-key-endpoint-store`
- Type: `feature`
- Revision: 1
- Status: proposed

## Goal and scope

An initialized Mac gateway installation has one gateway-owned Keychain item whose entire secret value is a randomly generated 32-byte master key. All persisted local gateway configuration, identity and credential state lives in private authenticated encrypted files. This replaces the baseline requirement to store native token values and checkpoints individually in Keychain. The central service remains the source of assignments, settings and recoverable upstream credentials.

## Functional Requirements

- FR-001: Include the selected gateway origin, native identity and session, policy snapshot and freshness checkpoints, static bearer keys, OAuth access/refresh tokens, pending OAuth saves, credential publication/rejection markers, install receipts and gateway recovery metadata. Only inert filesystem coordination and minimal nonsecret format/routing fields may remain outside encryption. Passwords are never persisted. The existing UI launch command, central APIs and agent configuration stay compatible.
- FR-002: Use OS randomness, HKDF-SHA-256 domain separation and authenticated encryption with unique nonces and versioned framing. Bootstrap encrypted identity without trusting an ID supplied by an agent or the unverified file. Bind account data to the verified server/deployment, user, installation and login generation; additionally bind each MCP credential to its intended MCP/runtime and binding. Keychain holds no other secret, checkpoint, marker or wrapped state in the completed format.
- FR-003: Preserve replay protection across same-boot offline restart, process races, logout, rebind and credential rejection, including when an entire previous file set is restored. A static key plus replayable encrypted files is insufficient. Use a single-slot protected key transition for commits that must invalidate an older state, or an equivalently demonstrated single-key design. Under the process lock, stage and sync new encrypted state before committing the key transition; competing readers must authenticate the currently committed generation. Never rely on a plaintext counter or a second Keychain item.
- FR-004: If a recoverable record fails authentication, deny its use and make a bounded authenticated pull for fresh authorized configuration/credentials. Publish the replacement under fresh local key material atomically, retaining healthy records where their integrity and authorization are provable. A rotated local encryption key does not rotate the provider's API key. Missing master key or unreadable login state requires independent authentication (interactive sign-in when necessary) before a rebuild. Distinguish a locked/unavailable Keychain from a missing key; never overwrite it on access error. If offline, revoked or denied, leave affected work unavailable. Recovery does not replay tool calls, installs, or OAuth exchanges; uncertain local-only state requires explicit recovery, never fabricated success.
- FR-005: Support current-format installations with a recoverable cutover using validated ownership and current authorization. Fence concurrent legacy writers or require their shutdown before migration. Finish preparing and verifying new storage before retiring old data; prevent rollback to the old format after cutover. If an old Keychain ACL requires the creating executable, provide an explicit supported handoff. Do not claim migration complete while owned legacy entries remain; never delete unrelated items. Logout invalidates live generations and cleans owned state even offline; repeat cleanup safely.

## Non-Functional Requirements

- NFR-001: Reuse existing safe Rust crypto, private paths/permissions, bounds, locks and zeroization where applicable. No raw secrets in ordinary files, temporary writes, logs, browser reads, agent config or protocol output. Preserve the existing trusted OS/Keychain boundary.
- NFR-002: Preserve the one-hour maximum lease, exact expiry, clock checks, current policy/credential checks, generation fences, 128-MCP bound and provider refresh version/lease checks. Repair alone never extends authorization. Keep bounded I/O and avoid expensive rewrites on each tool call where correctness permits.

## Acceptance Criteria

- AC-001: In an isolated real Mac Keychain namespace, exercise sign-in, sync, install receipt, bearer and OAuth calls, refresh/recovery and simultaneous UI/STDIO processes. Observe exactly one gateway item containing only master-key bytes in initialized operation. Inspect fixture files for recognizable plaintext secrets and configuration; the browser/agent receives only approved outputs.
- AC-002: Fresh process can decrypt valid state and use eligible tools within the existing outage window. Copies across identities/installations and old file or full-directory restores after revocation, rejection, logout or clock-watermark advancement cannot grant access. Deny at the earliest expiry, including exactly 3,600 seconds.
- AC-003: Corrupt, truncate or remove a record and demonstrate bounded online repair, renewed local key material and successful subsequent tool use without process restart when the original session remains valid. Test denial/offline, locked/missing/wrong key, login change during pull, and isolated failure for unrelated healthy credentials. Repeat high-impact operations only with explicit user action.
- AC-004: Inject interruption before/after file and Keychain transitions and during migration/cleanup. Fresh and already-running processes agree on the committed state or deny uncertain access. Exercise current legacy inputs, pending OAuth responses, rejected credentials and logout cleanup; demonstrate one item after successful cutover and no access through retired copies.
- AC-005: Run applicable locked Rust quality/build and security regressions, a supported Apple Silicon packaged-client rehearsal with scoped cleanup, and update the canonical setup, identity, cache, credential and recovery guides with tested behavior and limits.

## Dependencies

No prerequisite track. The merged Mac MVP and Dhivon baseline are the implementation starting point.

## Additional Information

This is local endpoint storage only. Plaintext already present in historical backups cannot be removed by migration. Central vault keys, OAuth provider profiles, server URL prompting, release signing and public distribution are outside this change.

## Dependent-track impact

No active dependents. Refresh project-level descriptions of the prior storage model through the normal Cadre refresh workflow after delivery.

## Approval

Pending combined specification and plan approval.
