# Product roadmap

Dhivon MCP Gateway is a Dhivon AI product. This roadmap distinguishes the delivered
baseline, current approved work and candidate directions. It carries no release
dates. A capability becomes delivered only when its artifact and verification
evidence are recorded.

## Verified baseline

The reviewed macOS MVP delivers a self-hosted control plane and Apple Silicon
endpoint with the following capabilities.

| Capability                                                     | What it enables                                                          | Evidence                                                                     |
| -------------------------------------------------------------- | ------------------------------------------------------------------------ | ---------------------------------------------------------------------------- |
| Central identities, groups, assignments and tool policy        | Administrators control who may use each MCP and original tool name       | [Administration](admin-ui.md), [configuration](configuration.md)             |
| Explicit per-user enablement and local management              | Developers choose which assigned MCPs to enable                          | [Mac setup](mac-install.md), [local management](local-ui.md)                 |
| One agent STDIO connection                                     | A gateway aggregates permitted local and remote tools                    | [Agent setup](agent-configuration.md), [aggregation](gateway-aggregation.md) |
| Supported remote authentication                                | None, owned bearer keys and the tested public-client OAuth profile       | [Integrations](integrations.md), [OAuth](oauth.md)                           |
| Bounded offline authorization and protected native credentials | Eligible work continues only within the existing valid lease             | [Security](security.md), [cache](endpoint-cache.md)                          |
| Self-hosted operations and isolated recovery rehearsal         | Operators can verify restart, fresh restore and independent-key recovery | [Deployment](deployment.md), [MVP release report](release.md)                |

The original release report describes the artifact it exercised. See
[compatibility](compatibility.md) for the exact platform, browser, protocol,
authentication and distribution limits. Controlled fixtures are the provider
baseline, and the local archive is unsigned and unpublished.

## Current approved delivery

The approved `dhivon-brand-site-docs` track is completing the Dhivon rename,
consistent branding across both management interfaces, a public product website
and this documentation set. Its acceptance requires current builds, browser
checks and a fresh artifact/deployment rehearsal. The site and documentation must
explain the access model and lead a reader through a first permitted tool call.

## Approved next work

There are currently no approved follow-on product tracks beyond the current
rebrand, website and documentation delivery. The directions below remain
candidates until separately scoped and approved.

## Directions under consideration

| Candidate                             | What it would enable                                                        | Evidence needed to call it delivered                                                                            |
| ------------------------------------- | --------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Signed and notarized Mac distribution | A trusted, repeatable installation channel                                  | Signed/notarized artifacts, published checksums and an installation rehearsal on the supported Mac versions     |
| Wider agent and provider validation   | Documented interoperability with named clients and real OAuth/MCP providers | Versioned compatibility entries and real discovery, call, refresh, expiry and disconnect evidence               |
| Additional endpoint platforms         | Teams can use Intel Macs, Windows or Linux where approved                   | Platform credential/process/clock design plus packaged end-to-end and failure-path evidence per OS/architecture |
| SAML identity                         | Operators can integrate an approved enterprise identity provider            | Defined identity/session mapping, role/revocation behavior and tested provider login/logout flows               |
| Hosted SaaS delivery                  | A managed deployment option                                                 | Approved tenant isolation, operations, key ownership, recovery and lifecycle design with verification           |
| Deployment scale and hardening        | Operators can assess larger or more demanding environments                  | Explicit load/availability targets, measured results and tested operational recovery                            |
| Broader MCP capabilities              | Clients can use selected protocol features beyond the current tools profile | Approved feature semantics and interoperability, authorization, resource-bound and cancellation tests           |

Feedback should describe the user workflow, intended environment and acceptance
measure. A candidate's presence here grants no implementation or availability
commitment. New supported capabilities are reflected in the compatibility guide,
operator instructions and release evidence when completed.
