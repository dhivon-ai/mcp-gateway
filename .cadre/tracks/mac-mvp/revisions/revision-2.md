# Revision 2026-10-15: mac-mvp

- Source status: ready_for_review
- Target status: in_progress
- Base commit: 892b94dd1c3efa0f0d74b382366d6b2fa0d25f0a
- Previous track revision: 1
- New track revision: 2
- Previous spec revision: 1
- New spec revision: 2
- Previous plan revision: 3
- New plan revision: 4
- Revision commit: pending

## Requested change

Implement BUG-005: user management has Name, Email, Password. User explicitly selected email-only login without backward compatibility because the system is not live, then clarified Name is display-only and Email is for login. Continue implementation autonomously under these concrete instructions.

## Proposed artifact changes

FR025/AC013, revised spec2/plan4, append P16 identity/browser/release work and P17 final verification; reassess learning/dependency context; track revision2. Email normalization/uniqueness and display bounds are explicit. Older populated identity databases are rejected safely; fresh pre-release deployment is required, with no automatic data deletion or username fallback.

## Affected work and provenance

Quiescent completed execution2026-10-14T23-41-00-967Z and all70completed nodes remain immutable. Preserve P1–P15 exact titles/tasks/markers/groups and all prior review/finding/attempt history. No workers/worktrees or partial product work. Pending dirty state contains only already-recorded clean-review evidence; retain it as history. Bug-intake commit892b94d is retained. No additive revert needed.

## Review and verification impact

Invalidate the old clean-completion proposal/readiness for product52c288b. Start a new execution for P16/P17 with inherited Autonomous/parallel modes after receipt reconciliation. Run current identity/security/native/browser/package/restore verification and cumulative review; final clean completion still needs explicit human approval. No approval of the previous completion proposal is claimed.

## Cascading dependency impact

None: track registry contains only mac-mvp; no dependencies/dependents or approved reusable patterns. Project remains self-hosted AppleSilicon/Rust/React/TypeScript/shadcn; SAML/SaaS and other release exclusions unchanged.

## Approval

Authorization comes from the user's explicit implementation instruction and subsequent email-only/no-backward-compatibility/display-name decisions in this chat. This record does not claim approval of final completion or authorize deletion of any real data. Receipt binds the exact revision artifacts and derived transition.
