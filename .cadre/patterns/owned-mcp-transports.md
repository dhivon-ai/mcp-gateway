# Pattern: Bounded MCP transports with owned process shutdown

## Use when

A STDIO gateway aggregates untrusted processes and authenticated remote MCPs with
current policy, cancellation and bounded resource use.

## Guidance

- Share effective-policy gates across discovery and dispatch; use stable aliases
  over MCP/tool IDs with policy expressed in original upstream names. A route is
  not lasting authority: recheck protected policy and credential readiness.
- Audit the pinned SDK's typed/raw seam. Bound frames, HTTP/SSE bodies, schemas,
  pagination/cursors, tools, startup and concurrency. Omit unsupported task-required
  tools and advertise only capabilities actually implemented and tested.
- Disable automatic reinitialization/replay for tool calls; reject redirects and
  pin authenticated destinations. Never retry potentially side-effecting calls.
- Own child process groups and discovery jobs, using abort-on-drop ownership.
  Keep the leader unreaped until the final group TERM/KILL to prevent targeting a
  recycled PID; reap only the owned child within one shutdown bound.
- Keep protocol descriptors nonblocking/cancellable under idle input and output
  backpressure. Mac agent hosts use FIFO pipes and connected AF_UNIX stream pairs
  (Node child_process); reject files, terminals, datagrams and TCP descriptors.
  stdout is protocol-only; logs exclude payloads and credentials.

## Tradeoffs and limits

Verified with rmcp3.5.1/MCP2025-11-25 tools-only and actual Mac clients. Reassess
SDK defaults/extensions when upgrading. Cancellation cannot undo side effects;
group ownership does not supervise deliberately escaped descendants or guarantee
cleanup after SIGKILL. Darwin zombie-only EPERM needs exact process-state proof,
not blind acceptance. Other OS/commercial-agent/provider support requires tests.

## Provenance

- Track: `mac-mvp`; clean completion approved in `42e27c246d7869a94ed67098eb3da4bc6a395532`.
- Phase/task: P3/T3.6, P4/T4.1–T4.4, P8/T8.2.
- Commits: `3c0de2000a830cf788851658164d366b2585cb10`, `38a858e23fde380bf37a7658367715e96cd333e2`, `fa504880b4d89cd26df3661c6ddab9bf5c981a2d`, `4469f1aaba87f2f1d447900da28afc2a78bfe860`.
- Source/evidence: CLI owned_process/native_stdio/runtime/aggregation/upstream_http/wire modules; gateway/protocol/SDK aggregation tests for signals, TERM-ignoring descendants, pending startup, socket pairs, malformed/paged tools, cancellation and admission.
- Archived at: 2026-10-15 (proposed archive batch).
