# Incremental Learning: Dhivon MCP Gateway rebrand, product website and documentation

Read this seed at each root phase. Later phases read the learning from every declared dependency phase. Append verified observations and handoffs during execution.

<!-- cadre:pattern-seed:start -->
## Pattern Seed

<!-- cadre:memory:start -->
{"schemaVersion":1,"specRevision":1,"planRevision":1,"patterns":[{"path":"patterns/authorization-leases.md","sha256":"a949846db0786cbcc3b82b93a073f6a0427175fc8c02033d7fcc771db11360c8"},{"path":"patterns/browser-contract-consistency.md","sha256":"85449fd4523c3f17cb692d2cfaea42497ef85870884f2795d963fe4a17d6931d"},{"path":"patterns/credential-publication.md","sha256":"28346d9e885a27b8d0334138788cb40eaee2f1b1f395b670d33cf1bd74dfb7b3"},{"path":"patterns/isolated-release-verification.md","sha256":"3453134357e4efd47faaca9080982f997ea6485ff7b9297504f11339e85dbd5a"},{"path":"patterns/owned-mcp-transports.md","sha256":"d407bb97d2b46c96f9b109e6c18af6fcb769a2d4ed52051c4d4d6f8f808083a3"}]}
<!-- cadre:memory:end -->

- Spec revision: 1
- Plan revision: 1

| Pattern | Relevance to this track | Constraints to carry forward |
| --- | --- | --- |
| `patterns/authorization-leases.md` | Security and troubleshooting guides explain revocation and offline use. | Only a complete durable sync renews the lease. Enforce the earliest deadline, pinned generation and per-call policy. Expiry denies use and offline mutations are rejected. |
| `patterns/browser-contract-consistency.md` | Both UI shells and forms change. | Keep coherent revisions, explicit conflict reload, lossless argument/environment editing, backend-aligned decoding, stable account identities and keyboard/secret controls. |
| `patterns/credential-publication.md` | Fresh native namespaces and remote-auth guides touch ownership and cleanup. | Preserve protected heads, scoped Keychain accounts, bounded metadata, preflight and retirement journals. Preserve complete OAuth CAS recovery, token lifetimes, origin pinning and metadata-only UI reads. |
| `patterns/isolated-release-verification.md` | CLI names, packaged assets and docs all change the release inputs. | Test the extracted binary and actual image in generated private scopes. Retain the creating CLI through verified logout. Exercise restart, fresh restore, independent keys and redaction; label limits. |
| `patterns/owned-mcp-transports.md` | The public diagram, docs and agent rehearsal describe the actual gateway. | Document the tested tools-only profile, bounds and alias mapping. Preserve per-dispatch policy, protocol-only stdout, no automatic replay and ownership-safe child cleanup. |
<!-- cadre:pattern-seed:end -->

## Phase learning

Awaiting execution.
