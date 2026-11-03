# Consumer setup verification, November 3, 2026

Internal verification for the guides and root Compose stack at product commit `0a6cdafe0da93bd7bc00dd430840d06406261390`. The public site uses an explicit list of guides and excludes this record.

| Acceptance | Result |
| --- | --- |
| Find the right guide | In the connected Chrome profile both Get started links led to the correct Mac or operator guide. Desktop 1440×900 and mobile 390×844 checks followed both paths, verified keyboard navigation, and found no page-level overflow or browser errors across the guide directory, quickstart, deployment, Mac and agent pages. Full-page homepage screenshots and link text/destinations matched the baseline at both widths. |
| Publish consumer material | The isolated build check inspected all 33 emitted files, links, anchors, redirects, and deliberately unlisted guide, image and brand fixtures. The old bug, design and historical release routes returned 404 in preview. The current release-evidence URL redirected to user-facing checksums. Project and third-party notices matched source in the site, final Mac archive and runtime image. |
| Start and recover the server | On Docker 29.8.2 / Compose 5.5.0, verified a fresh root deployment, first administrator sign-in via a local HTTPS reverse proxy, private PostgreSQL, stable repeat starts and full restart. Interrupted PostgreSQL initialization and first-admin creation recovered. A fresh logical restore resolved a saved credential with matching retained material. A different valid vault key refused startup. Key rotation preserved access after the previous key was retired. |
| Build the CLI | On Apple Silicon macOS 27.0.1, with Node 24.19.0, pnpm 11.4.0 and Rust 1.95.0, a fresh checkout built without output exports. Explicit cache/target overrides also passed and preserved unrelated files. The final archive included the updated install and compatibility guides, embedded console and exact notices. Its SHA-256 was `5450c6aaffab666458d0d2ce8f146f912103923c84021bd549e476b227c9bf8f`. |
| Use an assigned tool | With a new root Compose project, the final extracted CLI served its local console, signed in as an assigned user, installed/enabled an echo server, and returned the message through one agent STDIO connection. A call to the same alias after disablement was rejected. The creating CLI logged out, retired its isolated Keychain item and the temporary Docker project was removed. |

The guide paths are rendered independently of the authenticated applications. The HTTPS check used a local test certificate trusted by the test client. The separate native tool journey used the documented isolated loopback test exception. Neither test provisions public DNS or certifies a named commercial agent. The local public-site preview was used for the Chrome review.

Reproduction checks include `bash scripts/verify-compose-setup.sh` with an owned build directory, `pnpm --filter @dhivon/mcp-gateway-site typecheck`, `pnpm exec prettier --check apps/site`, `pnpm site:build`, `node apps/site/check.ts` and the emitted notice checker. Server rustfmt, strict Clippy, ordinary server tests and all 11 isolated PostgreSQL-backed cases passed.

## Release blocker verification, November 3

The operator changes were verified at `44704dd`. Mac packaging used the repeat-build changes at `56af458`, the offline guide packager, and the same canonical deployment and backup guides. The verified archive is identified by its SHA-256 below.

| Case | Verification |
| --- | --- |
| Linux first start and restart | In the pinned Postgres 18.6 linux/arm64 image, the operator owned the private directory (0700) and password (0600) as UID 1001. PostgreSQL ran as UID 999. Two starts on Linux tmpfs accepted authenticated TCP queries and retained the same password and cluster identity. |
| Database URL disclosure | The built server and managed image passed top-level help, subcommand help and two invalid invocations without the synthetic password in stdout or stderr. |
| Configuration interrupted or contested | Killed the helper before and after private publication. The same checkout recovered. Concurrent checkouts, a second config location and a foreign directory could not replace the owner’s inputs. The eight setup regression cases passed on Linux and macOS. |
| Invalid email or origin | A double-dot email and port 99999 failed before settings publication. From the actual guide, a plus-addressed administrator signed in at an explicit HTTPS port. The private deployment survived a restart and authenticated reads of a saved bearer credential. |
| Backup failure and restore | Immediate failure, partial output, interruption, name collision and repeated success preserved earlier dumps. A real stopped-database export also preserved the prior archive. Restored the successful owner-only dump to fresh PostgreSQL, invalidated copied sessions and verified the same saved credential through HTTPS using the matching separately held vault. |
| Repeat Mac build | With pinned tools on Apple Silicon macOS 27.0.1, two default builds took 58.34 s and 25.52 s. The second compiled one crate. Both used the same 767,248 KiB Cargo target and produced independent release directories. Each extracted CLI served the expected UI for that build. Explicit output overrides preserved unrelated files. |
| Archive onboarding | The final unsigned archive had SHA-256 `8d40699ffb6480488edad6460fd57c2d566112596371145f81ffe7db1aac4383`. Extracted outside the checkout, 37 local links and anchors resolved across all 14 bundled guides. Checksum, notices and version passed. From the extracted CLI the assigned echo tool returned its message, and the same alias was denied after disable. The creating CLI logged out and retired its isolated Keychain item. |

Both guide journeys passed keyboard and navigation checks in Chrome at 1440×900 and 390×844. Homepage screenshots and destinations matched the baseline. The static publication check covered 33 files and the retired ticket returned 404. The HTTPS test used a local test certificate and the native STDIO test used the isolated loopback fixture described above.
