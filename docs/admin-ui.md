# Administrator website

The central Rust service serves the compiled React/TypeScript/shadcn website at
its configured public origin. Sign in with an explicitly bootstrapped administrator.
Ordinary users receive an administrator-access message; Rust independently rejects
their administrator API requests. See [identity.md](identity.md) for bootstrap and
session rules and [configuration.md](configuration.md) for policy semantics.

## Manage the deployment

- **Catalog:** add a structured STDIO command or Streamable HTTP endpoint. New
  entries have no assignments and an empty allowlist. Configure optional local
  installation commands separately; this website never executes them. Remote
  authentication is none, a user-selected bearer secret, or a pre-registered public
  OAuth client ID and scopes. Credential values belong to the user's vault.
- **Access:** add user, group or all-user assignments. Grants are additive; losing
  the last grant clears enablement and later reassignment requires a new opt-in.
- **Tools:** select original observed names or enter names manually. Observations
  are untrusted inventory, not permissions. Allowlist hides new names until selected;
  denylist allows unlisted names for an otherwise eligible MCP.
- **Users / Groups:** create accounts, edit roles/active state, reset passwords,
  and select group members by stable identity. Account/password changes revoke
  sessions. Destructive catalog/group actions require a confirmation naming their
  effect. The last active administrator is protected by the service.

Changes use the revision at editor opening. A conflict retains the edit and offers
**Reload current state**; it never silently resubmits with a newer revision. Saved
means the server confirmed the write. If the follow-up read fails, the interface
requires a reload before another write. Identity and group reads are bracketed by
configuration reads with the same global revision. A concurrent change produces
explicit reload feedback before any editor can pair stale values with a newer
revision. The same consistency check protects the read after a confirmed save.

Runtime and installation arguments use JSON arrays of strings; environment values
use JSON objects of strings. Empty arguments, escaped newlines and literal characters
remain unchanged when saving another field or tool policy. For example, `["-c",
"print(1)\nprint(2)", ""]` contains three arguments, including an empty final argument.
MCP names may contain up to 200 UTF-8 bytes. Catalog decoders share the server's byte
limits, including long environment keys within the 1 MiB catalog budget.

Passwords clear after requests. CSRF remains in memory; browser sessions are
HttpOnly cookies. No native session tokens, bearer/OAuth values or endpoint
credential APIs are exposed by this website. Invalid network JSON is rejected by
runtime decoders, and requests have a ten-second deadline.

## Build and serve assets

Use the execution-provided build cache while implementing with Cadre:

```sh
GATEWAY_BUILD_CACHE="$BUILD_CACHE" pnpm build
GATEWAY_WEB_ASSETS="$BUILD_CACHE/web" CARGO_TARGET_DIR="$BUILD_CACHE/cargo" cargo build -p gateway-server
```

The resulting binary embeds its index and assets; it needs no Node process at
runtime. The build rejects symlinks, unsupported types and excessive asset counts
or size. Without compiled assets, the index returns an actionable 503; APIs and
operator commands remain available. Assets are served from an exact embedded
lookup, with no runtime filesystem traversal. Same-origin security headers include
no-store, nosniff, no-referrer and a restrictive CSP. Inline styles support Radix
positioning; inline scripts and external origins are disallowed.

## Verification and design fidelity

The flow checked is central login → real catalog/policy/assignment/account/group
actions → server-confirmed state, explicit conflict recovery and role denial.
The disposable fixture script creates one unique database inside the owned
`cadre-gateway-tests` PostgreSQL environment and one private key directory. It
accepts no production DSN. Run with the owned build-cache environment above:

```sh
GATEWAY_BUILD_CACHE="$BUILD_CACHE" CARGO_TARGET_DIR="$BUILD_CACHE/cargo" node scripts/admin-browser-fixture.ts
```

The synthetic username is `admin`; the fixture-only password is
`owned-browser-fixture-password`. This is not a deployment default. SIGINT cleans
up its own child process, unique database and private files. Owner metadata permits
exact recovery after interruption and contains no credentials.

Verified on macOS 27.0.1 arm64 using Codex's in-app browser: production assets
served by Rust, 1586×992 desktop, 1200×900 laptop and 390×844 narrow views, real policy saves,
structured command creation, assignment creation, account creation, group
membership edits, logout/re-login, ordinary-user denial, empty search results,
real concurrent revision conflict and explicit reload. A long unbroken catalog
name wraps safely; the table's minimum width preserves action-column spacing.
Required empty-field feedback appears before submitting a write. Keyboard checks cover tab
focus inside sheets, Escape and return to the opening control. Narrow tables
scroll inside a labelled, keyboard-focusable region; the page does not overflow.
The group editor keeps its save controls available below scrollable content.
No relevant console errors or framework overlay appeared. Component tests cover
CSRF/password clearing, no ordinary-user admin reads, filtering, manual tools,
membership IDs, conflict without retry and destructive-action cancellation.
Frontend tests (10), lint, typecheck, formatting and production build pass.

Native renders were compared with all three [design references](design/admin/README.md).

| Comparison | Render and resolution |
| --- | --- |
| Canvas / palette | True white canvas, cool gray rail, forest green controls; no decorative imagery or metrics. |
| Desktop layout | 312px rail, 48px main gutters, 435×50 search, 56px header and 88px catalog rows; corrected initial control sizing and column drift. |
| Typography | Geist Variable, 40px page heading, 30px sheet heading, deliberate 18px controls and 16px support text. |
| Editors | 678px MCP / 560px membership sheets, scrollable body and retained footer; small control-spacing differences follow shadcn composition. |
| Icons / selection | Lucide outlines plus a code-native three-node gateway mark matching the reference; eligibility dots have accessible text and do not imply remote health. |
| Manual tool input | Add sits in InputGroup's trailing addon instead of a separate adjacent box, preserving the shared shadcn control pattern. |
| Dynamic membership | All actual accounts, including the administrator, appear; groups use server order. A modal leaves the underlying full-width table intact. |
| Narrow layout | Compact brand/navigation/account area, full-width sheets and horizontal table regions continue the same design. |

Above-the-fold copy matches the catalog reference: brand, four navigation names,
title/subtitle, Add MCP, Search MCPs, column labels, Manage, account and Sign out.
Fixture rows now use stable ordering for comparison; real deployments use actual
data. Actions has an accessible-only column heading. Tool-policy reference copy
is preserved; its denylist helper deliberately describes newly permitted names.
Group editor copy is preserved, with real administrator membership added. Required
login, user, connection, assignment, validation/loading/error states extend the
same component system. No unrelated product area was introduced.

This evidence does not claim Safari/Firefox coverage, older macOS validation,
hosted CI execution, endpoint installation or remote OAuth execution; those
endpoint and release acceptance checks belong to their later implementation phases.

## Review remediation verification

The cumulative review added Rust-compatible UTF-8 catalog bounds, lossless JSON
command editing and revision-bracketed administration reads. The compiled Rust
websites were exercised again on macOS 27.0.1 arm64 in the in-app browser at
1584×993 and 390×844. A backend-valid 200-byte name and 129-byte environment key
loaded in both websites; policy-only save preserved runtime/installation arrays
and maps exactly, including empty strings and embedded newlines. Invalid command
JSON was rejected before saving, and a valid explicit edit succeeded. Long sheet
headings truncate visually while retaining their complete accessible names and
keeping the close control available. Narrow sheets retain reachable actions with
no page overflow; Escape returns focus to the opening control.

A second fixture administrator removed a group member during an open edit. The
stale save returned 409 and explicit full reload showed the member still removed.
Deterministic interaction tests additionally interleave both group and account
changes during initial loading and post-save reads: inconsistent data never opens
an editor, confirmed mutations are not replayed, and read failures require reload.
Local native login, assigned long-name management, confirmed enable/disable and
logout passed. Both consoles were free of relevant errors/overlays. Final screenshots
remain outside the repository. All 34 frontend tests, strict TypeScript, lint,
formatting and embedded production builds pass. This controlled fixture evidence
does not extend the previously declared browser/provider/platform compatibility.

Administrator user, group and assignment responses have no server row-count cap.
Their decoders validate every record without imposing a smaller browser-only cap;
large valid collections remain loadable. Explicit wire bounds still apply to
individual group membership (10,000), MCP definitions (512), command arguments
(128), and tool names (2,048). Regression tests cover 10,001-record coherent reads,
malformed late rows and real PostgreSQL/API responses above the former limit.

The final collection correction passes all 41 frontend tests and the added real
PostgreSQL/API regression. Strict TypeScript, lint, formatting and embedded builds
pass; previously verified layout/keyboard interactions are unchanged.
