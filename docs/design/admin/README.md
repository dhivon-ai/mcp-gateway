# Administrator interface design

T6.1 design baseline for the approved mac-mvp specification, accepted by the
coordinator under persisted policy-v2 Autonomous authority. This is a design
artifact, not a claim that the website is implemented or browser-tested.

The identity fields have changed since the original design. Sign-in uses
Email and Password; user creation requires Name, Email and Password, and edits
keep Name (display-only) separate from Email (sign-in). User tables show both.
Current implementation and browser evidence are in `docs/admin-ui.md`.

## References and visual system

ImageGen produced three coordinated references. Each has been inspected with
`view_image`; all are 1586 × 992. The PNGs are references only: every live label,
control, row and icon is rendered in React, with no screenshot-based interface.

| Reference                                    | Production surface                                          | Priority           |
| -------------------------------------------- | ----------------------------------------------------------- | ------------------ |
| [catalog.png](catalog.png)                   | Administration shell, catalog table, search, primary action | First viewport     |
| [tool-policy.png](tool-policy.png)           | Right editor, tabs, policy radios, observed/manual names    | Dense editor state |
| [group-membership.png](group-membership.png) | Groups table, membership editor, destructive action         | Membership state   |

The primary canvas is true white. The sidebar is pale cool gray; forest green
is reserved for primary actions, selected navigation and checked controls.
Approximate semantic token targets: background/popover/card `#ffffff`, foreground
`#111923`, muted foreground `#677078`, sidebar `#f2f6f6`, sidebar accent `#e0ece9`,
primary `#124d3e`, primary foreground white, border/input `#dce3e6`, destructive
`#d92335`. Verify these against the reference at native dimensions; do not warm
the white canvas or introduce a gradient. Shadows are limited to dialog/sheet
separation. No charts, marketing imagery, metrics, pill labels or decorative cards.

Use the existing Geist Variable font with system sans fallback. At reference
width: main heading 40px/48px semibold, panel heading 30px/38px semibold, section
heading 24px/32px semibold; body/control/nav 18px/26px, supporting text 16px/24px,
brand 23px/30px semibold. Form labels are medium; table names semibold. Font sizes
must be deliberate on controls and navigation, including mobile. Use 8px spacing
rhythm with 12px/16px for internal alignment, 24px between fields, 36–48px gutters.

The desktop sidebar occupies approximately 312px of the 1586px reference; catalog
content starts at x362 with a matching right gutter. The sidebar is viewport-height
with its account/sign-out region at the bottom. The page header places title and
subtitle on the left and one primary action at the right. Search is about 435px
wide and 50px high. Table header height is 56px and catalog rows about 88px; use
one outer border with horizontal separators, restrained header tint and 6px radius.
The tool sheet is about 678px wide with 36px internal gutters; the group sheet is
about 560px wide. Sheets scroll their body and retain the footer below a separator.
Match the reference geometry at 1586px; scale gutters/sidebar intentionally for
smaller laptops, rather than using a different density everywhere.

Icons use the existing Lucide outline family, currentColor, rounded strokes
about 2px: Network brand, FileText catalog, LockKeyhole access, UserRound users,
UsersRound groups, LogOut, Search, Plus, X, Trash2 and checks. Navigation icons
are approximately 24px; brand mark approximately 40px; control icons 20px.
The account avatar is a circle with a first-letter fallback. No standalone raster
art asset is needed because this is a code-native icon and typography interface.

The only overlay is the shadcn Sheet/Dialog backdrop for an open editor; it dims
the underlying interface neutrally and does not tint the white sheet. There are
no media overlays, image masks or background imagery. Motion is limited to
short sheet/focus transitions and obeys reduced-motion preferences.

## Allowed copy and necessary extensions

Reference copy, including order: MCP Gateway, Administration; Catalog, Access,
Users, Groups; account name, Administrator, Sign out; MCP catalog; No access
until assigned. Users choose what to enable.; Add MCP; Search MCPs; MCP,
Transport, Tool policy, Access; Manage. Data labels: STDIO, Streamable HTTP,
No authentication, Bearer token, OAuth, No tools allowed, All tools, Unassigned,
All users, and a count followed by allowed or blocked. Fixture names Local tools,
Remote tools, OAuth tools and Engineering are test data, never hardcoded rows.

Tool editor: Connection, Access, Tools; Tool policy; Allow selected tools;
Block selected tools; New tools stay hidden until selected.; Observed tools;
Add tool name; Original upstream name; Add; Enter names manually before discovery.;
Cancel; Save policy. Observed echo/fail are untrusted fixture names.
Group editor: Groups; Group membership adds access to assigned MCPs.; Create group;
Group; Members; Edit; Choose who belongs to this group.; Group name;
Delete group; Cancel; Save group. Engineering/Documentation/alice/bob are fixtures.

Required functional extensions in this same visual system:

- Sign-in form: Sign in, Username, Password, Sign in; context fetch/loading,
  fixed actionable login error, and ordinary-user Administrator access required.
- Users table/editor: Users, Create user, Username, Role, Status, User,
  Administrator, Active, Disabled, Edit, Save user, Reset password, New password.
  Password fields are never prefilled and clear after a completed request.
- Connection tab: Name, Description, Active, Transport, Executable, Arguments,
  Environment, Working directory, Authentication, Endpoint URL, Public client ID,
  Scopes, Installation command, Save MCP, Delete MCP. Review remediation uses JSON
  string arrays for runtime/installation arguments and JSON string maps for their
  environment, preserving empty strings and embedded newlines. Scopes remain one
  per line. These fields never execute commands from the central website.
- Access table/tab: MCP, Assigned to, Add assignment, Remove assignment,
  All users, User, Group, Select user, Select group. Explain additive grants and
  that removing the last grant clears enablement. There is no central Enable button.
- Operational feedback: Loading, Retry, No MCPs yet, No users yet, No groups yet,
  No assignments yet, No observed tools, No results, Saved, Changes need review,
  Reload current state, plus fixed field-validation and permission feedback.
  A 409 retains the edit, explains concurrent changes and asks for explicit reload.
- Destructive confirmation uses AlertDialog naming the actual target and effect;
  action and cancellation are explicit. Policy deletion is not an inert decoration.

Intentional deviations: colored catalog dots represent active assigned eligibility,
not upstream health (the central service does not probe execution). Include a
text/accessibility equivalent and show disabled status when relevant. Dynamic
rows/copy come from real responses. The block-mode helper changes to explain that
new unlisted tools are permitted. Long assignments may show a bounded summary
with details in Access. Required empty/error/login/connection/user/assignment states
extend the pictured component families; they introduce no extra product areas.

## Workflows and API ownership

The shared generated Rust/TypeScript contracts are authoritative. Runtime-decode
every response before use; compile-time types alone cannot validate network JSON.
The client sends same-origin cookies, rejects redirects, and keeps CSRF in memory.
It must support 204 without parsing JSON, preserve validated revision information
on 409, and mask malformed errors. Never persist tokens, passwords or CSRF in
localStorage/sessionStorage. Never call native endpoint credential routes here.

| Journey                           | API/contract                                                                                                     | Confirmed behavior                                                                                             |
| --------------------------------- | ---------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| Login/context/logout              | `/api/context`, `/api/auth/login`, `/api/auth/logout`; UiContext/LoginRequest                                    | Fresh preauth CSRF, HttpOnly browser cookie, role gate; revoke stale cookie must allow a fresh login challenge |
| Catalog/assignment/inventory read | GET `/api/admin/configuration`; AdminConfiguration                                                               | Definition metadata, additive assignments, bounded untrusted observed names                                    |
| Catalog create/edit/delete        | POST `/api/admin/mcps`, PUT/DELETE `/api/admin/mcps/{id}`; CatalogWrite/RevisionInput                            | expectedRevision on every write; new entry empty allowlist and no grants                                       |
| Add/remove assignment             | `/api/admin/assignments`, `/api/admin/assignments/{id}`; AssignmentWrite/RevisionInput                           | Explicit target; no assignment means no access; show server-confirmed result                                   |
| Users create/edit/reset           | `/api/admin/users`, `/api/admin/users/{id}`, `/api/admin/users/{id}/password`; UserCreate/UserEdit/PasswordReset | Revision-checked; role/active/password changes revoke sessions; last active admin protected                    |
| Groups/membership                 | `/api/admin/groups`, `/api/admin/groups/{id}`; GroupInfo/GroupWrite/RevisionInput                                | Choose stable member IDs; deletion reconciles grants centrally                                                 |
| Tool policy                       | CatalogWrite definition.toolPolicy plus inventory                                                                | Original names only; allowlist new names hidden; observations never grant access                               |

Read identities/groups concurrently between two configuration reads and require
the same global revision before opening editors. Review remediation applies this
consistent read-set check to initial and post-save loading; a conflict requires
explicit full reload. Never blindly retry an admin write with a newer revision.
Forms use current domain IDs, not row indexes.
API paths and exact methods are checked against central router source during
implementation; all mutations remain enforced by the Rust service.

## Components, accessibility and layout

App is composition glue. Separate shell/navigation, session boundary, shared API
decoders and loading/errors, catalog editor, access editor, users and groups.
Use shadcn Button, Input/InputGroup, Field/FieldGroup/FieldSet/FieldLegend,
Table, Sheet, Tabs, Select with SelectGroup/SelectItem, Checkbox, RadioGroup,
Alert, Empty, Avatar/Fallback, Skeleton, Separator and AlertDialog as needed.
Reuse semantic tokens and existing button variants; no manually recreated overlays.

Tables have scoped headers and labelled row actions; search has a real label.
Every sheet has an accessible title/description, focus trapping, Escape and focus
restoration. Checkbox groups use a fieldset/legend; radio choices have labels.
Errors associate with inputs; request status uses aria-live. Loading controls are
disabled with labelled spinner, and success follows confirmed responses.
Use native forms/buttons/links; never raw HTML from upstream names/descriptions.

Below laptop width reduce sidebar and gutters. Below 768px replace the rail with
a compact brand and four labelled navigation tabs; account/sign-out remain reachable.
Sheets become full-width with scrollable content and an accessible footer. Tables
retain their table model with labelled horizontal scrolling where necessary; no
card-grid replacement. Test at 1586×992, a laptop viewport and mobile 390px, with
long names, empty data, network/error/forbidden/conflict states and keyboard-only use.

## Verification contract

Design review maps these journeys to FR-022/AC-001/002/011 and NFR-005. Current
contracts contain all necessary public fields; secrets belong to the later local UI.
T6.2/T6.3 must run interaction tests and real-server browser journeys, then compare
native-size renders to all three references using view_image. Record a fidelity
ledger for copy, layout, typography, palette, icon/spacing/container treatment and
mobile continuation, plus an above-the-fold copy diff. Product completion requires
actual compiled assets served from Rust and no fixable visual or functional gaps.
