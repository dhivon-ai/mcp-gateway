# Pattern Catalog

Patterns are distilled from completed, cleanly reviewed tracks during archival.
Read applicable bodies and their limits before applying guidance to new work.

| Pattern | Summary and applicability | Provenance | Supersedes |
| --- | --- | --- | --- |
| [Authorization leases](authorization-leases.md) | Endpoint outage authorization: complete-only renewal, protected identity/generation/receipt, conservative deadlines and per-dispatch gates. | mac-mvp P1/P3/P4; clean review42e27c2; task commits in pattern. | None |
| [Credential publication](credential-publication.md) | Protected/ordinary stores: immutable publication, bounded metadata, complete-response OAuth CAS recovery, rejected expired-token refresh ordering and scoped cleanup. | mac-mvp P2/P3/P5/P10, BUG-001; dhivon-brand-site-docs P8/T8.1; task/review commits in pattern. | None |
| [Browser contract consistency](browser-contract-consistency.md) | Revisioned forms: coherent read sets, backend-aligned bounds, lossless commands and stable identity/profile separation. | mac-mvp P11/P14/P16, BUG-002–005; task commits in pattern. | None |
| [Owned MCP transports](owned-mcp-transports.md) | MCP aggregation: bounded untrusted I/O, no side-effect replay, current policy and ownership-safe nonblocking shutdown. | mac-mvp P3/P4/P8; task commits in pattern. | None |
| [Isolated release verification](isolated-release-verification.md) | Native/self-hosted release: immutable artifacts, exact emitted notices, scoped cleanup/fresh restore and HTTPS/native-callback evidence with provider limits. | mac-mvp P8/P12/P14/P16; dhivon-brand-site-docs P5/P7/P8, BUG-006/007; task/review commits in pattern. | None |
| [Rehearsal recovery ownership](rehearsal-recovery-ownership.md) | Verification harnesses: retain child ownership before fallible registration, fail closed on uncertainty, hold exclusive crash-retryable recovery through final evidence. | dhivon-brand-site-docs P10/P12, BUG-008–010; commits03764af/9aba60c, clean review7249c26; full provenance in pattern. | None; complements isolated release verification and owned MCP transports. |
| [Canonical product guides](canonical-product-guides.md) | Public docs/brand delivery: one guide source, concrete role/control story, honest roadmap, local shared assets and rendered navigation/CSP evidence. | dhivon-brand-site-docs P1–P5; task commits and clean review7249c26 in pattern. | None; complements browser contracts and isolated release verification. |

Existing patterns are extended where guidance overlaps; none is merged, retired
or superseded. All contributing track/task/commit provenance is retained in the
bodies. There are no non-completed, non-archived tracks to reseed or rebase.
Completed and archived learning remains historical evidence, including its old
pattern hashes; those historical hashes do not claim current-pattern freshness.
