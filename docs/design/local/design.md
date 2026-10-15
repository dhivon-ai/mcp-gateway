# Local management design

T7.1 design specification for FR-023. Generated with the built-in ImageGen tool, inspected before implementation. These references guide code-native React/TypeScript/shadcn interfaces; no screenshot is shipped as UI. The existing vector GatewayMark is the brand authority.

BUG-005 supersedes identity copy in the historical reference images below: login
uses Email (`you@company.com`) and Password; the account label uses display name.
Name never authenticates. Current compiled-browser evidence is in `docs/local-ui.md`.

## Reference inventory and visual lock

- `catalog.png` is the primary shell/table reference (1584 × 993). White main background, 350px pale sidebar, 58px main gutter, 84px top gutter. Heading 48/58, body 22/30, muted secondary 20/28. Table header 60px, rows 106px, buttons 58px, radius 6px. At narrower widths reduce gutters/type and retain an accessible horizontally scrollable table.
- `bearer-sheet.png` specifies the right-hand connection sheet only. Its invented background rows/icons are excluded: retain the catalog reference underneath. Sheet 530–560px, 30–34px gutters, title 40/48, body20/28, divider-separated open sections, bottom full-width action. Exact visible sections: Connection; Secret key; Choose a key; Manage secrets; Tool access; Enable MCP. Helper: Choose a key from your encrypted secret store. Values are never shown here. Footer: Enablement is saved on the server. Changes require a connection.
- `secret-sheet.png` specifies secret CRUD and the write-only Add secret sheet. Heading Your secrets; Encrypted on the server. Values are write-only.; Add secret. Table Key / Version / Actions. Actions Rename / Replace value / Delete. Form Key / Value; Values cannot be viewed after saving.; Save secret. Footer Secret values are never returned to this browser.
- `login.png` specifies the open sign-in form: Sign in on this Mac; Connect to your enterprise MCP Gateway.; Server; Username; Your username; Password; Sign in; Your password is never saved on this Mac. Retain the primary shell's 350px rail and existing triangular mark rather than the generated image's inconsistent rail and rings.
- `status.png` specifies open server/configuration/revision/validation/expiry rows and Coding agent setup. Retain primary shell geometry/nav icons. Exact status copy: Connection status; Configuration and authentication on this Mac.; Refresh; Offline · Read-only; Cached configuration expires in [countdown]. Reconnect to make changes.; Server; Configuration; Revision; Last validated; Expires; Coding agent setup; Configure one STDIO entry for your coding agent.; mcp-gateway mcp; At expiry, the gateway stops exposing tools until a complete refresh succeeds.

Palette: true white #fff; sidebar #f2f6f6; ink #111923; muted #677078; border #dce3e6; forest #124d3e; selected navigation #e0ece9. Geist Variable. No gradients, decorative cards, charts, metrics, window chrome, raster assets or marketing. Existing shadcn semantic tokens provide colors; local geometry is scoped to avoid changing the central website. Portaled sheets inherit local scope explicitly.

Icons: existing connected-circle vector mark; lucide List / LockKeyhole / Activity / LogOut / RefreshCw / Search / X / ChevronDown / Copy / TriangleAlert, outline rounded stroke2, nav30px, controls24px. State dots are semantic inline indicators with text, never color-only meaning. Avatar has a letter fallback.

Catalog allowed copy: MCP Gateway; This Mac; MCPs; Secrets; Status; alice (actual username); Sign out; Your MCPs; Choose what your coding agent can use.; Refresh; Online; Configuration expires in [countdown]; Search MCPs; MCP / Connection / State / Actions; Local tools / Local STDIO / No authentication / Disabled / Install / Manage; Remote tools / Streamable HTTP / Bearer token / Disabled / Manage; OAuth tools / Streamable HTTP / OAuth / Disabled / Manage; Only MCPs assigned to you are shown. Your administrator controls tool access. All names, rows and status values are live data, not a seeded product catalog.

## Interaction and API ownership

The native local page identifies its mode through a non-secret HTML data attribute. Central admin rendering remains separate. `/api/context` contains redacted native status and CSRF; no credential values. Generation headers pin GET/logout; every mutation wraps `{expectedGeneration,input}`. The UI never stores authentication/configuration/secrets in browser storage.

Login clears passwords after every attempt and reloads native context. A changed generation unmounts all prior identity state and aborts stale reads. Logout clears local presentation immediately after confirmation and reloads the page to establish a fresh local cookie. A one-hour browser cookie can be renewed only by reloading the local page, independently of the policy lease.

Catalog and redacted status poll while visible, refresh on focus/reconnect, and bound rendered countdown by both native remaining time and absolute expiry. Native code remains authorization authority. Expired/unvalidated caches show no usable catalog. Offline valid cache is read-only; all install/settings/secret/OAuth writes and vault reads require online preflight. No mutation queue or optimistic enablement. Errors/conflicts require explicit Reload current state before reconsidering edits; never blindly replay writes.

The Manage sheet reuses bearer-sheet geometry for each auth mode. No-auth has no secret selector. Bearer lists only current-user key metadata and saves stable IDs. OAuth shows connected state separately from native ready/rejected/expired/sync-required status. Connect/Reconnect opens provider consent only after explicit action; retain a cancellable pending flow, poll status, then require explicit Enable MCP. Disconnect confirms current version and disablement; cancellation is explicit. No raw authorization URL/token is rendered as diagnostic text.

Installation is an explicit sheet action with explanation that the administrator's command runs as this Mac user. Show native progress/outcome without command arguments/output. Optional install-and-enable is explicit; installed receipt and centrally enabled state are separate. Failure/unknown commit never claims enablement; retry enable does not rerun installation. Reinstall requires explicit confirmation. Cancellation is available while running. These functionally required states use the same Sheet/Alert/Button families rather than inventing another major surface.

Secret create/replace inputs are passwords, write-only, bounded, cleared after each attempt and unmounted on close/identity change. Rename and delete use record-version CAS; deletion confirms the key and explains active binding restrictions. No secret value is loaded for editing.

Tool access shows original allowlist/denylist names; empty allowlist is explicitly blocked. Runtime diagnostics distinguish missing executable, install failure, auth required/rejected/expired, unavailable central service, and blocked tools. Enabled means central opt-in only, never an assertion that an upstream is healthy. Catalog browsing never launches an upstream/installer.

## Verification gates

Meaningful interaction tests cover login, identity change, real confirmed settings, stale revision recovery, secret selection/write-only CRUD, install failure/retry, OAuth consent/cancel/disconnect, offline and exact expiry states. Use IAB first for compiled native UI against controlled real services, keyboard/focus, desktop/native reference size and mobile390px. Inspect concepts and latest browser screenshots through view_image in the same fidelity pass; record five or more comparisons, copy deviations, console and core journey evidence in the local UI runbook before completion.

## Generation prompts

Built-in ui-mockup briefs requested complete flat desktop local application screens with white/forest/pale sidebar, Geist, practical React/shadcn components, exact copy above, no tokens, commands, invented metrics or new navigation. Primary catalog refinement removed OS chrome and restored pale selected navigation. Separate bearer, secret, login and offline-status briefs kept the same palette/typography/component families. Differences explicitly excluded above are generated inconsistencies, not alternative product designs.
