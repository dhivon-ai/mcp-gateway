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

## Phase 1 — Product identity and approved assets

Renamed Cargo packages/directories, binaries, native namespaces, cookies, build/deploy environment keys and developer tooling in 757d8b169cedc6578742c8a99e10fea45b24e1d1. The approved brand kit and TTF embedding support arrived in bb057261035dadea4a830e5ef66e9a092bae6ce0, integrated by 3ca90793f24df65b376e2b3930f7c4babf248351.

Locked workspace check/build, embedded frontend production build, generated contracts, frozen pnpm install, formatting, 44 frontend tests, fresh Compose configuration/start and CLI version/help passed. Native cache and identity suites passed. Full native gateway passed on a stable-binary retry after an initial run overlapped a rebuild. PostgreSQL tests needed their safety guard renamed with the isolated fixture database. Extended OAuth verification is still running.

Keep exact original dependency pins. Dated historical evidence retains the identities it actually tested. Emerald against white is 2.90:1 and is an accent. Use navy actions and darker semantic success text. Both asset packers support TTF. Shared assets live in assets/brand and need copying into the Docker web build stage when consumed.

An initial native gateway failure retained its private fixture journal. Reconcile using the creating binary before removing anything. Final release rehearsal must use an immutable extracted binary and record new artifact identity. Current project metadata branding requires the planned post-track Cadre refresh.

## Phase 2 — Administrator and developer interfaces

Shared/admin branding is committed in 03a0ae79e333cae95b3260ca8e3419df797d254b. Local identity and removal of unused Geist are committed in 2d2b662d1885b44b389687c545ebddb32a49b08a. Manrope, navy controls/sidebar and actual approved light/dark SVGs are bundled locally.

In-app Chromium exercised compiled admin login/catalog/editor, keyboard Escape/focus return and desktop/390px layouts. Native local login, status, installation feedback, server-confirmed enable and disable passed, with no relevant console errors or document overflow. Vite SVG data URIs initially failed CSP. External SVG imports fixed image delivery without weakening CSP. Final local sidebar typography correction is in source and will be checked again with packaged P5 assets.

Frontend 44 tests, lint, type/build pass. Native Rust suites passed; rustdoc subsequently observed deleted old web chunks while frontend output was rebuilt. A stable final-source rustdoc retry passes. Extended central/native OAuth passes sequentially in 30.93 seconds. Final checks must keep asset inputs and binaries stable for their entire run.

## Phase 3 — Product and operator documentation

T3.1 delivered README and role index, source-to-first-call quickstart, local/remote auth recipes and runnable Node fixture client in 3a927a3643fa8b435d8c197230ccef2361d5309c. T3.2 added architecture, security, symptom-specific troubleshooting, contributor and undated roadmap guides plus corrected current deployment/compatibility/identity/endpoint-management references in 3c5d4f3b65fffa45674c054ee1a6c0ee7905cbbd.

Canonical top-level Markdown is the site publication input. Guide links/anchors, shell/JSON/Node syntax, CLI options and runtime configuration were checked. The Node client called the SDK fixture using a protocol-only wrapper. Root's rebranded container and extracted Mac rehearsal then passed deployment/bootstrap, explicit install/enable, one agent STDIO call and disable denial, restart, fresh restore, independent wrong/matching keys and log redaction. P5 records final identities and any changed packaging inputs.

Historical release/acceptance/design/bug reports keep their original tested names. Roadmap labels the current delivery, has no approved follow-on track and gives candidates concrete outcomes/evidence without dates. Add a current release-report link only after that file exists.

## Phase 4 — Public product and documentation site

The static apps/site workspace renders canonical Markdown through pinned Marked 18.1.2 and copies the shared official brand kit. Commit 4a3ccc03f1334237204fc8ba23b328bf414241bc implements the public-site group. Outputs are separate from embedded management assets, with scoped build/preview commands.

Three Image Gen section concepts informed the white/navy editorial layout. Implementation uses original SVGs and dark links, omitting invented tool categories. IAB desktop/390px verified the rendered homepage, quickstart navigation, roadmap direct links/refresh, local Manrope, asset loading and no relevant console errors or document overflow. Whole-site crawl generated 31 pages and found a stale identity anchor, fixed in canonical source. Static typecheck, frozen install, build and formatting pass. P5 repeats link crawl after final report publication.

The public site can be served independently with the control plane and native management stopped. It explains the need, ownership and execution locations and concrete echo grant→enable→call→disable flow. Hosting and physical checkout administration follow the approved later handoff.

## Phase 5 — Release rehearsal and Definition of Done

Release verification and current evidence are committed in fe930285c9c18bafbab36a72635b600aba47c7b5. The audit also corrected stale development fallback branding and added a packaged-CLI override to the existing local browser fixture. CI now includes public-site checks.

Passing evidence includes 83 Rust workspace tests, 11 explicitly run PostgreSQL cases, 44 frontend tests, formatting/lint/type/Clippy/build and generated-contract checks. The public build contains 32 pages with zero internal link/anchor failures. Compiled browser checks cover all three experiences, desktop/390px, keyboard and role-specific behavior. Packaged local sign-in, missing bearer credential gating, mark/font delivery and mobile status navigation pass.

The final archive SHA-256 is 2467f67ededf51f41e14e4c542d392995ea3be29965bfe37c61538259a41f275. Its executable is byte-identical to the extracted tested binary d7a870f6f64a70272afe092d78472e44febc13714341eeeb1fd43804ab68ff68. The service image ID is sha256:31d889b0612b8e049d454d7c658e379ec5d1504a56f31662587256df21570e5b. The extracted all-auth agent suite passes in 32.03s. The final image/binary rehearsal passes container bootstrap, actual Keychain login, explicit install/enable, Node STDIO call, disable denial, restart, pg_dump/fresh pg_restore, independent wrong/matching keys and redacted logs.

All generated rehearsal stacks/volumes and native credentials are cleaned through their owning fixtures. The early retained failure journal was logged out, confirmed credential/publication/cleanup lists empty and removed. Local-only static preview remains on port 4173. The runtime cache retains deliverable archive/checksum, site and test evidence.

Current product source is complete. Track mode now requires explicit human track-level verification before execution_finish and ordinary review. Post-track project metadata refresh, physical checkout naming and hosting follow the approved handoff. Preserve historical Cadre and release identities.
