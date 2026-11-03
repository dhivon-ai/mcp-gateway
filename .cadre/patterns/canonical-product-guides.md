# Pattern: Canonical product guides and evidence-bound claims

## Use when

A repository publishes a product website, operator/developer documentation and
branded protected interfaces for the same self-hosted/native product.

## Guidance

- Keep one maintained source per guide and render it into an independently built
  public site. Keep public output separate from authenticated embedded bundles;
  direct guide links, refresh and assets must work with product services stopped.
- Lead with the user's concrete configuration/access problem. Explain who owns
  policy, who opts in and where work runs. Use a source-aligned example from grant
  through enable, discover/call and denial after disable, with expected outcomes.
  Link role-specific entry points to setup, troubleshooting and compatibility.
- Label verified capabilities, approved next work and exploratory directions.
  An empty approved-next section is honest; a candidate is not a commitment.
  Bind availability/platform/provider claims to dated artifact evidence. Preserve
  historical names/hashes and publish current identities separately after rename.
- Vendor approved brand sources and fonts locally with their original notices.
  Share assets/tokens across surfaces, keeping role labels and semantic controls.
  Test actual emitted asset URLs against the existing CSP; external SVG emission
  solved the observed data-URI block without weakening that policy. Brand accent
  colors need separate contrast checks before becoming text, controls or status.
- Crawl rendered links, heading anchors, deep paths and copied assets after final
  evidence edits. Check compiled public/admin/native surfaces at desktop and narrow
  widths, including keyboard/focus and error states. Source type/build success
  alone does not prove embedded asset or navigation behavior.

## Tradeoffs and limits

The observed static renderer consumes repository-owned trusted Markdown; it is
not a sanitization design for arbitrary user HTML. The current site assumes root
hosting; subpath deployment needs explicit routing/asset verification. Tested
browser/layout checks cover their recorded environment, not every device or
accessibility technology. Reassess roadmap status when a later approved delivery
changes it; archival itself does not rewrite product pages or publish hosting.

## Provenance

- Track: `dhivon-brand-site-docs`; clean completion approved in
  `7249c26ed16da52cdcb3b9a848832e579ef7ab18`.
- Phase/task: P1/T1.3, P2/T2.1–T2.2, P3/T3.1–T3.2,
  P4/T4.1–T4.2, P5/T5.1–T5.2.
- Commits: `bb057261035dadea4a830e5ef66e9a092bae6ce0`,
  `03a0ae79e333cae95b3260ca8e3419df797d254b`,
  `2d2b662d1885b44b389687c545ebddb32a49b08a`,
  `3a927a3643fa8b435d8c197230ccef2361d5309c`,
  `3c5d4f3b65fffa45674c054ee1a6c0ee7905cbbd`,
  `4a3ccc03f1334237204fc8ba23b328bf414241bc`,
  `fe930285c9c18bafbab36a72635b600aba47c7b5`.
- Source/evidence: `apps/site/build.ts`, `assets/brand/README.md`,
  `apps/web/src/components/gateway-mark.tsx`, `docs/quickstart.md`,
  `docs/roadmap.md`, `docs/dhivon-release.md` and phase learning. Independent
  rendered-site and compiled admin/local desktop/390px checks, external SVG CSP
  correction and final34-page zero-broken-link/anchor evidence are recorded.
- Archived at: 2026-11-02 (proposed archive batch).
- Supersession: none. Complements browser-contract-consistency for UI semantics
  and isolated-release-verification for artifact/notice/deployment proof.

## Publish only consumer guides

- Select public guides and assets explicitly. Add an unlisted fixture and crawl
  the emitted files, links, fragments, redirects and notices. Keep internal bugs
  and verification records out of the public navigation and build.
- Exercise the two reader journeys from the actual entry links. For distributed
  guides, open the extracted archive outside the checkout and check its relative
  links and anchors. Put the final archive checksum outside its own inputs.
- Track `consumer-website`, P2/T2.1–T2.3 (`0a6cdaf`, `a1b88eb`),
  P5/T5.2–T5.3 (`9e353dd`), clean review `6ea16ed`. Evidence:
  `apps/site/publication.ts`, `apps/site/check.ts`, `scripts/package-guides.ts`
  and `docs/consumer-setup-verification.md`. The archive check covered 37 links.
  A clean local build does not establish what a hosting cache is serving.
