# General Coding Styleguide

- Read every existing file and directly relevant context before editing it.
- Prefer the smallest cohesive change that satisfies the approved plan task.
- Preserve established repository conventions unless an approved revision changes them.
- Make invalid states difficult to represent; validate at boundaries.
- Handle errors deliberately and keep failure messages actionable.
- Test externally visible behavior, failure paths, and meaningful edge cases.
- Keep comments focused on intent and tradeoffs rather than restating code.
- Avoid unrelated cleanup in delivery commits.

## Project-specific additions

- Preserve Rust safe-code linting and strict TypeScript; shared public contracts
  use generated TS plus runtime decoding, never unvalidated network assertions.
- Count wire limits as the backend does (including UTF-8 bytes); preserve literal
  structured arrays/maps rather than lossy display conversions.
- Use stable IDs for ownership; keep display names, canonical login identifiers,
  central enablement and native availability as distinct concepts and types.
- Model deadlines, generation/revision/receipt checks and publication/cleanup
  ownership explicitly. Errors never claim a server write or credential publication
  succeeded before confirmation; retries must not replay side effects.
- Match persistence reader/writer invariants and capacity checks. Keep credentials
  write-only in public flows, with fixed safe errors and redacted diagnostics.
