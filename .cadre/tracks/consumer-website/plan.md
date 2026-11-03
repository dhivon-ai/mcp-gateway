# Plan: A simpler product website and two-command setup

- Track: `consumer-website`
- Spec revision: 1
- Plan revision: 1
- Status: proposed

## Phase 1: Simplify setup commands

- Phase dependencies: none

### Outcome

An operator can launch the server stack and a Mac developer can produce a working CLI with one command each after the documented prerequisites.

- [ ] T1.1 Deliver the root Compose entry point and private first-run initialization; verify fresh start, interrupts, safe reruns, TLS, admin sign-in, private database, restart and restore. Record exact inputs, commands and results.
  - Task dependencies: none
  - Commit group: compose-setup
- [ ] T1.2 Give the Mac packaging command a safe owned output default and useful output paths. Preserve overrides. Build from a clean checkout and validate archive, executable, UI and notices. Record prerequisites and outputs.
  - Task dependencies: none
  - Commit group: mac-build
- [ ] T1.3 User Manual Verification

- Phase completion commit: pending

## Phase 2: Design the public experience and verify delivery

- Phase dependencies: P1

### Outcome

First-time visitors understand the value, find their own setup path and complete it from the published instructions.

- [ ] T2.1 Rewrite homepage, navigation and canonical start guides. Show one concrete use case, one primary action and two short audience paths using verified Phase 1 commands. Move detailed operations to secondary references.
  - Task dependencies: none
  - Commit group: consumer-site
- [ ] T2.2 Make publishing explicit for pages and assets, remove internal copy and links, preserve supported deep links and user-facing release facts. Cover exclusion with a regression check, crawl the build and verify notices and 404s.
  - Task dependencies: T2.1
  - Commit group: consumer-site
- [ ] T2.3 Run site formatting, typecheck and build, rendered desktop/390px keyboard/navigation checks and both documented journeys including first tool call and disable. Record screenshots, evidence and remaining limits, and check all acceptance criteria against the finished output.
  - Task dependencies: T2.2
  - Commit group: onboarding-verification
- [ ] T2.4 User Manual Verification

- Phase completion commit: pending

## Phase 3: Track-level User Manual Verification

### Outcome

Review the rendered landing page and both paths, the two working commands, credential-safe reruns, and evidence for AC-001 through AC-005 before accepting delivery.

- [ ] T3.1 User Manual Verification

- Phase completion commit: pending

## Approval

Pending combined track approval.
