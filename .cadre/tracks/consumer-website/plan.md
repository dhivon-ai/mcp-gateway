# Plan: Simpler product guides and two-command setup

- Track: `consumer-website`
- Spec revision: 2
- Plan revision: 3
- Status: planned

## Phase 1: Simplify setup commands

- Phase dependencies: none

### Outcome

An operator can launch the server stack and a Mac developer can produce a working CLI with one command each after the documented prerequisites.

- [x] T1.1 Deliver the root Compose entry point and private first-run initialization; verify fresh start, interrupts, safe reruns, TLS, admin sign-in, private database, restart and restore. Record exact inputs, commands and results. <!-- commit: a4d6c5b612cb3850794909113afbcd6c4cd8d736 -->
  - Task dependencies: none
  - Commit group: compose-setup
- [x] T1.2 Give the Mac packaging command a safe owned output default and useful output paths. Preserve overrides. Build from a clean checkout and validate archive, executable, UI and notices. Record prerequisites and outputs. <!-- commit: d047acbd1b54cdf90b1661e39a451784a7cf6c58 -->
  - Task dependencies: none
  - Commit group: mac-build
- [x] T1.3 User Manual Verification <!-- commit: 6414c595d6b72867d5a486c125ec6a22d5891eda -->

- Phase completion commit: `6414c595d6b72867d5a486c125ec6a22d5891eda`

## Phase 2: Simplify the detail pages and verify the setup journeys

- Phase dependencies: P1

### Outcome

Visitors entering from the existing homepage find their own setup path and complete it from the public guides.

- [x] T2.1 Simplify the documentation index, quickstart and linked detail guides. Give developers joining a team and operators starting a gateway separate short paths using the verified Phase 1 commands. Show prerequisites, the next action and expected result. Link advanced operations separately. Scope presentation changes to guide pages. <!-- commit: 0a6cdafe0da93bd7bc00dd430840d06406261390 -->
  - Task dependencies: none
  - Commit group: consumer-site
- [x] T2.2 Make publishing explicit for pages and assets, remove internal copy and links, preserve supported deep links and user-facing release facts. Cover exclusion with a regression check, crawl the build and verify notices and 404s. <!-- commit: 0a6cdafe0da93bd7bc00dd430840d06406261390 -->
  - Task dependencies: T2.1
  - Commit group: consumer-site
- [x] T2.3 Run site formatting, typecheck and build, rendered desktop/390px keyboard/navigation checks and both documented journeys including first tool call and disable. Compare the homepage with the baseline on both viewports and check its destinations. Record guide screenshots, evidence and remaining limits, and check all acceptance criteria against the finished output. <!-- commit: a1b88eb7bb4213806fdf869510e5a92f0eaee526 -->
  - Task dependencies: T2.2
  - Commit group: onboarding-verification
- [x] T2.4 User Manual Verification <!-- commit: a1b88eb7bb4213806fdf869510e5a92f0eaee526 -->

- Phase completion commit: `a1b88eb7bb4213806fdf869510e5a92f0eaee526`

## Phase 3: Track-level User Manual Verification

### Outcome

Review both detail-page journeys from the existing homepage, the two working commands, credential-safe reruns, and evidence for AC-001 through AC-005 before accepting delivery.

- [x] T3.1 User Manual Verification <!-- commit: a1b88eb7bb4213806fdf869510e5a92f0eaee526 -->

- Phase completion commit: `a1b88eb7bb4213806fdf869510e5a92f0eaee526`

## Phase 4: Repair operator setup and backup

- Phase dependencies: P3

- [x] T4.1 Fix BUG-001 and BUG-002. Prove first start and restart in the pinned database image with normal Linux ownership, then probe help and errors with a synthetic database password. Keep secrets private and verify first-admin HTTPS sign-in. <!-- commit: 72784c8638fa207b36ae4ac160657b5c2dd70af4 -->
  - Task dependencies: none
  - Commit group: managed-runtime
- [x] T4.2 Fix BUG-003 and BUG-004. Publish complete private inputs safely under interruption and contention; reject invalid admin email and HTTPS origin before publishing. Exercise interrupted and concurrent first runs, corrected inputs, and protection of an existing deployment. <!-- commit: 58541fcc4e8702a3735f13a5b36caea46c795de5 -->
  - Task dependencies: none
  - Commit group: configuration-recovery
- [x] T4.3 Fix BUG-005. Rehearse the exact backup guide from the checkout: fail before and after output, retain a prior good backup, verify private off-checkout output, then restore a successful dump with matching keys and read a saved credential. <!-- commit: bcbd099af26618aa783eccbde21d12f7e58e2b23 -->
  - Task dependencies: T4.1
  - Commit group: recoverable-backups
- [x] T4.4 User Manual Verification <!-- commit: 44704dd3ac0ddee74796fcc19859b3636de8da44 -->

- Phase completion commit: `44704dd3ac0ddee74796fcc19859b3636de8da44`

## Phase 5: Repair repeat Mac builds and archive onboarding

- Phase dependencies: P3

- [x] T5.1 Fix BUG-006. Build twice on the supported Mac, reuse Cargo compilation safely, measure target directories and disk usage, and verify the new archive embeds the current UI. Preserve release outputs, ownership and explicit overrides. <!-- commit: 56af458cc814b04d94b5417b02f103a8e82b43d9 -->
  - Task dependencies: none
  - Commit group: repeat-mac-build
- [x] T5.2 Fix BUG-007. Verify guide links including upgrade and agent setup from the final extracted archive outside the checkout. Verify checksum, notices, version and embedded console; keep the source and public guides canonical. <!-- commit: 9e353dd2ed72704de519000b5543ee22de0a0bad -->
  - Task dependencies: T5.1
  - Commit group: archive-guide-links
- [x] T5.3 User Manual Verification <!-- commit: 9e353dd2ed72704de519000b5543ee22de0a0bad -->

- Phase completion commit: `be6570c62bf88048a74f30545de77088e9410904`

## Phase 6: Track-level User Manual Verification

### Outcome

Verify all seven blockers against bugs/review-1.md. Record platform, commit and artifact identity; run both published journeys, including authorized first tool call and denial after disable. Recheck public links/404s, desktop/mobile detail pages and the unchanged homepage. Keep review tickets and internal evidence outside the public guides. Require evidence for every blocker before release.

- [x] T6.1 User Manual Verification <!-- commit: be6570c62bf88048a74f30545de77088e9410904 -->

- Phase completion commit: `be6570c62bf88048a74f30545de77088e9410904`

## Approval

Proposed in revision 2; see revisions/revision-2.md.
