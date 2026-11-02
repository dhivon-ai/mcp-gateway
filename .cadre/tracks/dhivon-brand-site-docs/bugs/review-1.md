# Review Bugs — dhivon-brand-site-docs

- Review cycle. 1.
- Status. Logging authorized by the product owner. Remediation implementation is deferred.
- Completed execution range. `470cbe7a5528219d1624c801b76ee596f266c5c3..eae1965b5154e701f2974c79d2b3d370d9aebc42`.
- Supplemental reviewed changes. OAuth clarification and source licensing through `d16c28a1ea2c63d56e1f1cca939ff7b0ef3faab1`, subsequently reverted at the owner's request before this transition. Product content is again identical to the completed execution.
- Review date. 2026-11-02.

## Coverage and verification

Reviewed the complete implementation range and its affected interfaces, including
Cargo/pnpm identities and lockfiles, native namespaces and cookie names,
deployment/packaging/fixture scripts, React branding, shared assets, static site
generation/preview, guides and roadmap. Compared all 68 Rust source, test,
manifest and migration files against the baseline with the explicit rename map.
Residual differences are formatting, TTF MIME support and development fallback
copy. The pure policy, lease, credential, OAuth and transport behavior is retained.

Reused the unchanged runtime evidence recorded by the completed execution.
It contains 83 passing Rust tests, 11 isolated PostgreSQL cases, 44 frontend tests,
compiled desktop/390px browser checks and exact packaged client/recovery checks.
Rechecked the actual archive manifest and TTF name-table metadata. The archive
contains the executable, README and compatibility guide, with no license/notice
sidecars. Its embedded font metadata has copyright and a license URL, without the
full OFL license text. Checked current source license metadata with Cargo metadata
and a frozen pnpm install before reversal. After reversal, Git verified that all non-Cadre tracked content exactly matches the completed execution HEAD.

The issues below are the consolidated open items. This pass confirmed one
additional release defect, BUG-007. BUG-006 remains the owner's deployment and
provider-verification follow-up. No additional runtime behavior defect was
confirmed in the changed policy, credential or transport paths.

## BUG-006 — Verify hosted HTTPS control plane with local OAuth callbacks

- Severity. P2 deployment/interoperability verification follow-up.
- Earlier report. Retained in Git at `8c9f8c7:docs/bugs/bug-006.md`; this consolidated entry supersedes it after the approved revert.
- Evidence. `deploy/Caddyfile.example:1-5` describes an HTTPS proxy but supplies a
  placeholder origin. `crates/dhivon-mcp-gateway-cli/src/main.rs:179-197` binds the
  local listener and connects the selected control plane.
  `src/management.rs:84-88` derives the callback from that listener.
  Current acceptance evidence uses controlled providers and a private HTTP hop.
- Impact. The intended architecture is supported, while an actual deployed
  HTTPS origin and named OAuth provider are still unverified.
- Reproduction. Inspect the configured deployment examples and release evidence.
  No real deployment origin/provider verification record exists.
- Remediation acceptance criteria. Record the approved HTTPS origin and named
  public native-client provider registration. Verify local-client sign-in/sync,
  browser redirect to its actual `127.0.0.1:{port}/oauth/callback`, S256 PKCE and
  state/issuer handling, credential persistence, explicit enablement, tool use,
  refresh and disconnect. Retain the legitimate local callback. Update the OAuth guide to clearly distinguish the hosted control plane from the native callback and document provider registration/port requirements.
- Prerequisites. The owner/operator supplies the actual hosting target, origin
  and provider configuration before this pending verification task can run.
- Authorization. The owner requested this record and subsequently clarified the
  HTTPS control-plane/local-callback topology. Implementation is deferred by the
  owner's instruction to log all bugs in the review cycle.

## BUG-007 — Project ownership and distributed license notices are incomplete

- Severity. P2.
- Evidence. `scripts/package-macos.sh:20-22` copies only the binary and two guides.
  `deploy/Dockerfile:24-25` copies the binary and healthcheck into the runtime.
  Both binaries embed the Manrope TTF through the frontend asset bundle.
  `assets/brand/fonts/OFL.txt:56-61` requires the copyright and full license to
  accompany redistributed font copies.
  `apps/site/build.ts:9-14` copies brand assets and CSS but does not publish the
  owner-authorized project LICENSE/NOTICE. After the approved revert, those root files and Cargo/npm license/ownership metadata are also absent.
- Impact. Mac/container recipients do not receive an easily accessible full
  Manrope license notice. The new Dhivon AI Apache-2.0 project notices also need
  implementation in source and propagation through the release/publication pipelines.
- Reproduction. List the actual
  `dhivon-mcp-gateway-0.1.0-macos-arm64.tar.gz` archive. It contains no LICENSE,
  NOTICE or OFL sidecar. Inspect the Docker runtime COPY list. Parse the TTF name
  records 0, 13 and 14 to confirm that the metadata contains a short license
  statement and URL, without the full OFL text.
- Remediation acceptance criteria. Add the official Apache-2.0 root LICENSE, Dhivon AI NOTICE/copyright, README attribution and Cargo/npm license/ownership metadata. Ship project LICENSE/NOTICE with Dhivon AI
  copyright and the unchanged third-party font copyright/full OFL in the Mac
  archive and container. Publish accessible project licensing/ownership notices
  alongside the static website. Add focused artifact-content checks, rebuild,
  verify the emitted notices and record the resulting archive/image identities.
  Preserve third-party attribution and the original reported artifact evidence.
- Source licensing decision retained; implementation pending. The owner selected
  Apache-2.0 with copyright Dhivon AI. Source update `d16c28a` was reverted by
  `78248da` at the owner's request so this review can transition against the
  unchanged completed execution. The license decision remains authorized.
  Both source licensing and distribution fixes belong to later remediation.
- Authorization. The owner explicitly requested logging all confirmed bugs
  together as part of the Cadre review cycle, with implementation deferred.

## Disposition

Record a changes-requested review cycle and append pending remediation phases.
Preserve all completed phases and the completed execution unchanged.
Do not start another execution, deploy a service or claim these issues resolved.

## Authorized reversal and transition evidence

The owner requested reverting the post-execution commits, then recording the
review transition. Four additive reversals preserve the original commits:

| Original | Reversal | Scope |
| --- | --- | --- |
| `525f002` | `ed758ce` | Fallback review report |
| `d16c28a` | `78248da` | Source license/ownership metadata |
| `8c9f8c7` | `9e623a3` | OAuth clarification and report correction |
| `91ec6e2` | `321b021` | Initial hosting report |

`git diff --exit-code eae1965 HEAD -- . ':!.cadre'` passed after the reversals.
Completed task and execution provenance remain unchanged. These reversals remove
the supplemental edits, while this report and pending plan retain the owner's
requirements. No remediation implementation or deployment is authorized now.
