# Pattern: Consistent administrator reads and lossless browser contracts

## Use when

React/TypeScript forms edit revisioned membership, account, profile or structured
command data whose correctness matters beyond the current screen.

## Guidance

- Generate public wire types and runtime-decode responses. Match actual backend
  UTF-8 and collection bounds; arbitrary smaller generic caps reject legitimate
  data. Decode every element and retain explicit limits on truly bounded fields.
- Bind all editor values to one revision. Configuration reads can bracket parallel
  user/group reads when every relevant write bumps a monotonic global revision.
  Apply this to initial and post-save loading; mismatches require explicit reload.
- Freeze the editor-opening revision and never blindly replay writes. Distinguish
  a confirmed write followed by failed synchronization from a failed write.
- Represent argument arrays and environment maps losslessly as JSON, with clear
  validation. Newline joining/splitting corrupts empty strings and literal newlines,
  including during policy-only saves. Preserve runtime and installation commands.
- Keep display names separate from canonical sign-in identifiers and stable IDs.
  Canonicalize/uniquely constrain email centrally; edits preserve stable ownership
  and revoke sessions. Recheck original login credentials under the final row lock
  after expensive password verification to fence concurrent rename/reset races.
- Use semantic controls, accessible feedback/focus and generation-keyed account
  state. Passwords are write-only/cleared after attempts; no credentials/CSRF in
  browser storage.

## Tradeoffs and limits

Revision bracketing requires complete participating writes and no revision reuse;
otherwise use an atomic backend read. Global revisions can conflict on unrelated
edits. Backend-valid collection support is not a performance guarantee. Email-only
fresh-database migration was explicitly authorized for a non-live prerelease; do
not transfer that compatibility decision to live accounts without authorization.

## Provenance

- Track: `mac-mvp`; clean completion approved in `42e27c246d7869a94ed67098eb3da4bc6a395532`.
- Phase/task: P11/T11.1–T11.2 (BUG-002/003/004), P14/T14.1 (BUG-002 follow-up), P16/T16.1–T16.2 (BUG-005).
- Commits: `829c6cd9ac24975bca68052861e8b3777042615f`, `c96f9c4abdc368331689f122f5abb301a9b62625`, `fabb61438e083dc031892291d6ad0ea0931e5661`.
- Source/evidence: `apps/web/src/lib/decode.ts`, admin read-data/catalog/identities components; real-PG identity/email/configuration tests; frontend race, 10,001+ collection and literal-command tests; compiled desktop/390px evidence in UI/release guides.
- Archived at: 2026-10-15 (proposed archive batch).
