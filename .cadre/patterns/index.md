# Pattern Catalog

Patterns are distilled from completed, cleanly reviewed tracks during archival.
Read applicable bodies and their limits before applying guidance to new work.

| Pattern | Summary and applicability | Provenance | Supersedes |
| --- | --- | --- | --- |
| [Authorization leases](authorization-leases.md) | Endpoint outage authorization: complete-only renewal, protected identity/generation/receipt, conservative deadlines and per-dispatch gates. | mac-mvp P1/P3/P4; clean review42e27c2; task commits in pattern. | None |
| [Credential publication](credential-publication.md) | Protected stores: one Keychain master, encrypted endpoint records, rollback and first-fetch refusal fences, independent recovery and OAuth CAS. | mac-mvp P2/P3/P5/P10, BUG-001; dhivon-brand-site-docs P8/T8.1; single-key-endpoint-store P1/P2/P5/P7, clean review aab0881. | Per-secret native Keychain placement for current-format endpoints. |
| [Browser contract consistency](browser-contract-consistency.md) | Revisioned forms: coherent read sets, backend-aligned bounds, lossless commands and stable identity/profile separation. | mac-mvp P11/P14/P16, BUG-002–005; task commits in pattern. | None |
| [Owned MCP transports](owned-mcp-transports.md) | MCP aggregation: bounded untrusted I/O, no side-effect replay, current policy and ownership-safe nonblocking shutdown. | mac-mvp P3/P4/P8; task commits in pattern. | None |
| [Isolated release verification](isolated-release-verification.md) | Native/self-hosted release: immutable artifacts, exact emitted notices, Linux UID and backup failure checks, HTTPS origin parity, repeat Mac builds and scoped native recovery. | mac-mvp P8/P12/P14/P16; dhivon-brand-site-docs P5/P7/P8, BUG-006/007; consumer-website P4/P5/P7; single-key-endpoint-store P3/P7. | None |
| [Rehearsal recovery ownership](rehearsal-recovery-ownership.md) | Verification harnesses: retain child ownership before fallible registration, fail closed on uncertainty, hold exclusive crash-retryable recovery through final evidence. | dhivon-brand-site-docs P10/P12, BUG-008–010; commits03764af/9aba60c, clean review7249c26; full provenance in pattern. | None; complements isolated release verification and owned MCP transports. |
| [Canonical product guides](canonical-product-guides.md) | Public docs/brand delivery: one guide source, explicit public files, role-specific journeys, rendered navigation and offline archive links. | dhivon-brand-site-docs P1–P5; consumer-website P2/P5, clean review 6ea16ed. | None; complements browser contracts and isolated release verification. |

Existing patterns are extended where guidance overlaps; the current-format native storage guidance replaces the earlier per-secret
Keychain placement. No pattern is retired. All contributing track/task/commit provenance is retained in the
bodies. There are no non-completed, non-archived tracks to reseed or rebase.
Completed and archived learning remains historical evidence, including its old
pattern hashes; those historical hashes do not claim current-pattern freshness.
