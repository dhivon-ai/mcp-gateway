# Compatibility and release limits

| Surface | Validated baseline |
| --- | --- |
| Endpoint | Apple Silicon arm64, macOS 27.0.1 (26A434); deployment target 27.0.1. Older releases/Intel/Windows/Linux endpoints are not shipped. |
| Native toolchain | Rust/cargo 1.95.0, edition 2024; macOS Keychain and continuous native clock/process APIs exercised on the actual Mac. |
| Website build | Node 24.19.0 LTS, pnpm 11.4.0, React 19.3.0, TypeScript 6.0.3, Vite 8.3.4, shadcn/Radix/Tailwind with committed lockfile. |
| Browser | Codex in-app Chromium on the validated Mac; desktop/laptop/390px flows. Safari/Firefox not independently validated. |
| Control plane | Rust/Axum/SQLx, PostgreSQL 18.6; self-hosted one enterprise per deployment. Linux arm64 container on Docker29.8.2/OrbStack and Compose5.5.0; fresh deployment/restore is recorded in release evidence. |
| MCP | Official rmcp 3.5.1 client/server, protocol 2025-11-25, STDIO (native pipes and Node-style Unix stream socket pairs) and Streamable HTTP. Tools/list, ordinary tools/call and list-change notifications only. |
| Remote authentication | None, owned per-user bearer secret, or preregistered public OAuth client with the supported discovery/PKCE/resource/issuer/refresh profile. Controlled providers only; see OAuth runbook. |
| Client/agent | Standard official rmcp client spawning only the CLI `mcp` entry. Commercial agent adapters are examples, not verified integrations. |
| Distribution | Local arm64 archive/checksum with embedded assets; not Developer ID signed, notarized or publicly published. |

No SAML, SaaS multi-tenancy, legacy HTTP+SSE, provider-specific grants, confidential
OAuth clients, resources, prompts, subscriptions, sampling, elicitation or tasks
are forwarded. Task-required upstream tools are omitted. Stable aliases preserve
routing for original MCP/tool IDs. Arbitrary vendor protocol extensions may not
survive typed SDK decoding.

Actual machine sleep/reboot and hosted CI have not been run. Deterministic
continuous-clock/boot/rollback tests plus an actual Keychain subprocess restart
cover conservative freshness. Unknown clocks/boot state require online sync.
The software trusts the OS/Keychain and does not resist privileged replacement.
Cancellation is best effort and cannot undo completed upstream side effects.
Owned process-group cleanup does not supervise deliberately escaped descendants.

Bounded MVP behavior: config TTL <=3600s, session <=12h, polling 60s/control10s,
install300s/startup30s/call60s/16 calls/shutdown5s. Discovery limits and HTTP/frame
bounds are specified in `docs/gateway-aggregation.md` in the source repository.
