# Endpoint snapshots, freshness and synchronization

A native login session does not itself permit tools. `SnapshotCache::latest`
reads the current shared committed snapshot and checks identity, integrity,
revision, session deadline and two clock deadlines before returning native policy
metadata. Future discovery and dispatch must call it for **every new request**;
there is no fallback to a previously loaded in-memory snapshot after a file,
credential or freshness check fails. Upstream readiness and the shared tool-policy
evaluator remain additional authorization requirements.

## Protected format and shared publication

`snapshot.enc` is an owner-only atomic file in the application's existing private
state directory. Format 1 is the eight-byte `MCPCACH1` marker, a fresh 12-byte
nonce and AES-256-GCM ciphertext/tag. The bounded file reader accepts at most
2 MiB plus 36 bytes. It reuses the core RustCrypto `KeyRing` implementation;
plaintext buffers and key material are zeroized on drop.

The encrypted payload contains format, complete `ConfigSnapshot`, canonical
origin/deployment/user identity, installation and generation UUIDs, unique receipt,
clock anchor and immutable native deadline. AEAD associated data independently
binds the format domain, identity, installation and generation. Schema, revision,
validation time and absolute configuration/session expiry are protected inside
the payload. Copied ciphertext cannot authenticate for another principal,
deployment, installation or login generation.

The snapshot AES key is a scoped record in the [single-key endpoint store](endpoint-identity.md#native-state-and-keychain-ownership). The authenticated metadata separately holds its receipt, revision, policy fingerprint, clock watermark and publication state.

Every transaction holds the shared state lock. An authenticated full response must match the current session, identity and generation; revisions and validation time cannot regress. Equal revisions must carry the same policy. The gateway publishes a pending checkpoint, writes the snapshot with a fresh data key, then publishes the committed checkpoint and rechecks freshness. Interrupted publication denies use until successful online synchronization. Losing the data key does not let repair bypass a healthy revision or clock checkpoint.

Reads publish a protected clock watermark without rewriting the snapshot or changing its receipt, validation time or expiry. The master key transition invalidates old indexes. Restoring an old snapshot or a full earlier set of ordinary files cannot roll the watermark back.

## Native clock and exact expiry

`NativeClock` uses safe `nix` 0.31.3 `clock_gettime` with Darwin's explicit
`CLOCK_MONOTONIC_RAW`, and safe `sysctl` 0.7.1 reads `kern.bootsessionuuid` when the
clock is constructed. It does not use Rust `Instant` as the authorization clock.
[Apple Libc's implementation](https://raw.githubusercontent.com/apple-oss-distributions/Libc/main/gen/clock_gettime.c)
maps RAW to `mach_continuous_time`, while
[Apple's XNU header](https://raw.githubusercontent.com/apple-oss-distributions/xnu/main/osfmk/mach/mach_time.h)
documents that continuous time advances during sleep. `CLOCK_UPTIME_RAW` uses a
different underlying clock and is not selected here. The maintained wrapper's
[safe function contract](https://docs.rs/nix/0.31.3/nix/time/fn.clock_gettime.html)
and [sysctl interface](https://docs.rs/sysctl/0.7.1/sysctl/) are the Rust API seams.

Freshness expires at the earlier of the server's absolute expiry and the native
continuous deadline. Server validity must be positive, at most 3,600 seconds and
bounded by the exact native central-session expiry. The request-start native
anchor and local wall time charge response/persistence delay against freshness;
publication cannot reset this budget. A clock check after persistence prevents
boundary-crossing Keychain work from returning an expired authorization.

The protected boot identity and native anchor survive process restart. The
protected watermark detects a wall-clock or continuous-clock regression since a
previous check, including after another process observed a later time. Missing
clock samples, boot change, regression or corrupt protected state fail closed.
A reboot requires a new online synchronization; a restarted process on the same
boot can use only the original remaining lease. Restored clock certainty alone
does not revive a cache already marked unusable. Detected clock uncertainty
requires complete online recovery with a valid current clock.

Deterministic tests permit access at 3,599 seconds and reject it at exactly 3,600
and 3,601. Earlier session expiry shortens this boundary. File rewrites, status
reads and upstream credential replacement never renew it. Sleep simulation
advances continuous time while freezing wall time and still expires the cache.

## Refresh, local integration and status

`NativeTransport` fetches `/api/config` through the native bearer session client,
with verified production TLS, no redirects and a 10-second transport deadline.
The entire fetch/body/parse operation also has a 10-second timeout. It requires
HTTP 200, bounded complete JSON and the full typed snapshot. Only validated,
durable installation renews freshness; partial responses, server failures,
credentials, reachability hints and failures do not renew it.

`SyncEngine::run` refreshes at startup, then schedules against native continuous
time every 60 seconds. A one-second wake loop notices elapsed time after wake;
it does not use a sleep-excluding timer as the policy clock. While authenticated
and offline it retries after one second between bounded failed requests, so a
responsive reconnection is observed promptly. `notify_reconnected` provides an
immediate native reachability/wake hint, performing a full refresh. Logged-out
processes avoid repeated polls until identity generation changes or login occurs.
There is no configuration mutation queue.

HTTP 401 or a complete same-identity inactive-account response clears local
identity/cache immediately. Permission failures, redirects, malformed responses
and outages remain distinct failures; any still-valid previous committed lease
may be used during an outage. Concurrent processes cannot overwrite a newer
revision or resurrect a logged-out generation with delayed network work.

`ui --server` explicitly selects its canonical control-plane origin before startup.
Same-origin selection preserves identity generation, owned credentials and the
existing cache. A different origin clears old authorization/cache/owned credentials,
changes generation (including any pending old login) and attempts old-session
revocation before requiring a fresh login. The ordinary selected-origin marker
also distinguishes two management processes when no account is currently active.
Each management/sync instance is bound to its configured origin. If another UI
selects a different origin, an older UI returns fixed `server_mismatch` metadata
without revealing or clearing the newer account; its requests/logout/credential
writes refuse that selection and its scheduler stops polling it. Only an explicit
login through that older UI may select its configured origin again. The management
router rejects a sync engine bound to a different configured origin.

Production `ui` attaches this engine to the protected management router, performs
startup/background refresh and attempts immediate full synchronization after
successful native login. Context/status responses contain explicit metadata only:
identity status, configuration state/revision/validation/expiry/remaining seconds,
last sync error category and the last refresh outcome (`online`). The outcome is
not permission to mutate: future writes still require authenticated online
requests and confirmed snapshot publication. `dhivon-mcp-gateway status` reads this
redacted shape without contacting the server. The agent-facing `mcp` command refuses startup without a valid policy snapshot and never opens a browser.

`authorization_changes` exposes a metadata watch channel for later runtime
expiry/revocation cleanup. Runtime consumers must still call `latest` before each
new discovery/call and arrange their own cancellation/deadline handling; a watch
notice does not replace dispatch authorization.

## Historical v1 verification

The evidence below describes the former per-item Keychain design. For current storage tests and the packaged upgrade rehearsal, see [testing](testing.md#single-key-endpoint-storage).

Tested on actual **macOS 27.0.1 (26A434), arm64, Rust 1.95.0**: native RAW samples,
stable boot identity across constructors, real Keychain AES storage, durable
ciphertext, separate-process decryption/watermark publication, parent reread
without resetting the deadline and owned logout cleanup. No system sleep or
reboot was triggered. Sleep/boot-change/clock rollback behavior is deterministic
fixture evidence plus the cited Apple clock semantics, not an actual suspended
machine claim. Older macOS versions and Linux endpoint releases are unvalidated.

An initial design updated one existing Keychain head entry. On this actual OS,
a child could update it successfully while the parent subsequently received
`errSecItemNotFound`; a fresh-process metadata lookup still found the item.
Immutable AES keys and fresh checkpoints avoid that observed cross-process
lookup/update behavior. No plaintext fallback or weakened freshness check was
introduced. Four exact owned stale fixture entries from that failed approach
were removed and verified absent using fresh-process Keychain CLI operations;
no unrelated entries or secret values were exposed.

The focused suites cover startup/60-second polling, unchanged renewal, reconnect
notifications, delayed revision/logout/origin races, same-origin preservation,
two-origin UI isolation, malformed/native HTTP failures,
401, exact expiry, earlier session expiry, clock uncertainty, ciphertext and
pointer replay, missing key, tampering, retirement/persistence failures and
expiry during publication. Captured status/log scans exclude fixture credentials
and parser bodies. The subprocess helper is marked ignored in ordinary discovery
but is explicitly run and asserted by its parent Mac test. Tests use uniquely
owned temporary state and fixture Keychain services; PostgreSQL was untouched.
