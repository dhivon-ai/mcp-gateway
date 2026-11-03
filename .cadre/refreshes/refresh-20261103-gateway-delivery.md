# Refresh 2026-11-03: Endpoint storage, deployment and public guides

- Status: proposed
- Previous refresh commit: `847218a35cb7461cbebcfb19bd71ac53814f6127`
- Evidence through: `e1ff21992a640046a147102744fd744477ff24f2`
- Target: Cadre 4.0.0, template set v6, schema 2
- Refresh commit: pending receipt

## Inputs and evidence

The two completed tracks were archived at e1ff219. The endpoint review at
aab0881 recorded packaged Apple Silicon concurrency, refusal and recovery tests.
The website review at 6ea16ed recorded root Compose and Mac packaging journeys,
fresh restore and public guide verification. Subsequent commits removed the duplicate
bug report (bdfb269), deployed the missing-doc Function (ad1daed), and delivered
content-versioned stylesheets (4ce8326). Production verification covered the exact
retired URLs and desktop/mobile guide navigation. This refresh uses those recorded
results and the current manifests, sources and guides. It adds no new product test run.

## Proposed context changes

Update product, engineering guidelines and tech stack for the one-item Keychain
store, authenticated repair, one-time operator setup, persistent Compose
deployment, Mac archive build, and published documentation with a scoped 404
Function. Distinguish public docs hosting from a self-hosted control plane.
Replace stale artifact identities with dated evidence links. Update the workflow
baseline and general and HTML/CSS conventions for guide distribution and cached
stylesheet compatibility. The existing language and framework guides still apply.

## Active-track impact

All four tracks are archived. All 14 historical executions completed. Only main
is registered, with no orphaned worktrees. All 24 distinct historical worker paths
and their recorded worker branches are absent. There are no active specs, plans,
seeds, dependencies, executions or approval modes to migrate. Preserve archived
history, prior approvals and all execution rules. The runtime records this refresh
in project history and lastRefresh under the approved receipt.

## Approval

Pending approval of this context-only proposal. No deployment or product change.
