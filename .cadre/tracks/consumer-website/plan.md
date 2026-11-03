# Plan: Simpler product guides and two-command setup

- Track: `consumer-website`
- Spec revision: 2
- Plan revision: 2
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

## Approval

Proposed in revision 2; see revisions/revision-2.md.
