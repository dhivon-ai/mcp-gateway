# Local management website

Run `dhivon-mcp-gateway ui --server https://gateway.example.com`. The Rust CLI opens a
loopback page and serves its embedded React/TypeScript/shadcn assets. No Node
process is required by the installed application. Sign in with your enterprise
email/password. The administrator website remains at the central origin.

## User journey

- **MCPs:** only assigned entries appear. A server grant does not install or enable
  anything. Manage shows central enablement separately from native installation,
  authentication and tool restrictions. An empty allowlist blocks every tool.
  Catalog browsing checks redacted executable metadata and never runs commands.
- **Install:** explicitly confirm the administrator-defined command. It runs with
  your Mac permissions; command output, arguments and environment are not shown.
  Choose enable-after-install only when intended. A successful installation and
  failed enable request remain separate outcomes. Reload current state and enable
  without rerunning installation. Reinstall requires a separate confirmation.
- **Bearer:** add a token under Secrets, then select its owned key in Manage.
  The stable secret ID is saved centrally. Downloaded credentials stay in native
  encrypted storage; a newly enabled entry may require native credential synchronization
  when the coding agent starts. Enablement does not claim remote health.
- **OAuth:** Connect opens the provider only after your action. The administrator
  must supply a supported preregistered public client ID; see [oauth.md](oauth.md).
  Pending-flow status never renews policy or races credential recovery against the
  callback. After confirmed consent, reload current state and explicitly enable.
  Reconnect performs new consent. Disconnect retires the local credential and
  confirms central removal/disablement. Cancellation remains explicit; an issued
  provider response or already-sent central request cannot be undone.
- **Secrets:** create, rename, replace and delete per-user keys with record-version
  checks. Values are write-only password fields, never loaded for editing, and
  clear after each attempt. Enabled bindings prevent deletion until disabled or
  removed. A conflict requires explicit reload/reconsideration; no blind retry.
- **Status:** inspect server origin, revision, validation/expiry and the generic
  STDIO command. Configure the absolute installed binary path in your coding
  agent as described in [agent-configuration.md](agent-configuration.md).

Offline configuration is read-only. The UI shows native remaining time and an
absolute-expiry countdown; the native gateway independently enforces the lease.
At expiry, no usable catalog is shown. No queued settings, installation, secret
edits or new OAuth connections exist. Reconnect/Refresh requests a complete
authenticated sync. Logout and cancellation can clear owned local work offline.

An enable/disable success appears only after the complete server response is
validated and durably published by Rust. A failure or unknown response may have
applied centrally: reload current state before trying again. Other clients arrive
through the native startup/60-second refresh loop. Neither credential refresh nor
an unsuccessful connectivity attempt extends the policy deadline.

## Browser and identity boundary

The page establishes a one-hour HttpOnly/SameSite local cookie; reload the page
when it expires. This cookie lifetime is independent of the policy lease and
central login session. CSRF stays in memory. Exact loopback Host/Origin checks
and generation-pinned APIs protect every action. Login generation changes unmount
prior account data and abort stale reads; out-of-order context responses cannot
restore a previous account. Logout reloads the local page for a new cookie.

No native token, cache key or secret value is stored in browser storage or read
APIs. Runtime decoders validate network JSON. Browser requests for multistep native
operations allow 90 seconds; individual control-plane requests remain bounded by
the native 10-second limit. Central admin requests retain their 10-second browser
deadline. Same-origin CSP forbids inline/external scripts; inline styles support
Radix positioning and focus handling.

## Build and test

Use the execution-owned build directory while implementing with Cadre:

```sh
DHIVON_MCP_GATEWAY_BUILD_CACHE="$BUILD_CACHE" pnpm build
DHIVON_MCP_GATEWAY_WEB_ASSETS="$BUILD_CACHE/web" CARGO_TARGET_DIR="$BUILD_CACHE/cargo" cargo build -p dhivon-mcp-gateway-cli -p dhivon-mcp-gateway-server -p dhivon-mcp-gateway-fixtures
DHIVON_MCP_GATEWAY_BUILD_CACHE="$BUILD_CACHE" CARGO_TARGET_DIR="$BUILD_CACHE/cargo" node scripts/local-browser-fixture.ts
```

The fixture accepts no production DSN/account, creates one owned PostgreSQL
database/private directory, copies the creating native binary for reliable
Keychain cleanup, and starts loopback central/local/MCP/OAuth services. Local UI
is at `http://127.0.0.1:18092`, central UI at `http://127.0.0.1:18091`.
Synthetic users `alice` and `bob` use `owned-browser-fixture-password`; this is
never a deployment default. The fixture bearer value is the public `fixture-token`.
SIGINT clears native state through its creating binary, verifies empty credential
journals, stops owned children and drops only its database/private directory.
Failed native cleanup retains its recovery files. Production HTTPS exceptions
are never inferred from fixture flags.

## Verified behavior and visual fidelity

The flow checked is compiled native sign-in → assigned catalog → explicit
installation/confirmed enablement → write-only secrets/bearer → actual OAuth
provider callback/explicit enable/disconnect → offline/read-only recovery →
logout/login as another user. The isolated service uses real PostgreSQL and Mac
Keychain, with controlled official-SDK MCP/OAuth fixtures.

Verified on macOS27.0.1 arm64 in Codex IAB at 1584×993, 1200×900 and 390×844:
real login, install then enable, current-user bearer selection, secret create/
rename/replacement, bound-key deletion rejection, disable then delete, actual
OAuth callback, explicit enable/disconnect, offline countdown with unchanged
validation timestamp, disabled offline edits, reconnect, empty search, Alice/Bob
isolation and logout. Tab stayed inside the sheet; Escape restored the opening
Manage button. Narrow tables scroll in a labelled keyboard-focusable region and
the document width equals the viewport. No relevant console errors or framework
overlay appeared. Fixed two first-pass geometry issues: catalog type/row rhythm
and a negatively positioned secret action. Secret actions now live in the actual
header, and the secret form uses its reference's narrower sheet.

The browser run exposed consent status polling refreshing the protected receipt
during callback persistence. Native pending-flow feedback now avoids recovery/
refresh during consent; complete issued-token publication retries only a bounded
local receipt change, never the provider exchange. The real controlled-provider/
PostgreSQL/Keychain regression passed (56.19s), including polling before and during
exchange, cancellation, version CAS, rotation and failed-save recovery. Native
catalog tests prove zero command execution/central mutation during browsing and
redacted missing-executable feedback. Frontend interaction tests cover confirmed
state, StrictMode, conflict without replay, secret-value clearing, owned selection,
install failure, OAuth cancellation, offline/expiry and late old-identity reads.

References: [design/local/design.md](design/local/design.md), generated with the
built-in ImageGen tool. Browser screenshots were saved outside the repository
and compared with the accepted references through `view_image` in the same pass.

| Comparison | Evidence and resolution |
| --- | --- |
| Catalog shell | 350px pale rail, 58px main gutter, true white canvas, three navigation rows; native-sized composition matches. |
| Type and rows | Geist Variable; corrected title to52px and row rhythm to104px; explicit22px controls rather than browser defaults. |
| Palette/icons | Forest controls, pale selected navigation, original vector mark and matching Lucide outlines; no gradients/cards/metrics. |
| Catalog copy | Header, subtitle, Refresh, search, four columns, actions and footer match; names/account/countdown are live values. |
| Bearer sheet |560px open sheet, owned key selector, original tool names, full-width enable action; real descriptions and diagnostic states are required dynamic content. |
| Secret sheet |450px sheet,34px heading, write-only fields and retained bottom action; actual empty/loaded data replaces sample rows. |
| Status |Open definition rows and one-entry STDIO setup; offline Alert, actual timestamps and failed-refresh feedback preserve truthful runtime state. |
| Responsive/focus |Compact navigation, wrapping header, full-width mobile sheets, labelled horizontal tables, trapped/returned focus and reduced motion. |

Above-the-fold copy was checked against the reference inventory. Intentional
differences: the primary catalog governs all shell dimensions and existing logo;
secondary generated backgrounds contain inconsistent logos/reflowed rows and are
not separate product layouts. Radix overlays leave the underlying main table at
its full width. Real user names, server origin, rows, auth diagnostics, errors and
countdowns replace sample data. Installation/OAuth/empty/expired states extend
the same required Sheet/Alert/Button families. The implementation was faithfully
verified against the accepted design; no material visual mismatch remains.

This phase does not claim Safari/Firefox, older macOS, hosted CI, external OAuth
providers or named commercial coding-agent interoperability. Release packaging,
deployment/restore and cumulative acceptance remain the integration phase.
