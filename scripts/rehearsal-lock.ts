/** One permanent inode, held by Node's descriptor; no PID-file reclamation. */
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { closeSync, constants, fstatSync, lstatSync, openSync } from "node:fs";
import { join } from "node:path";

// Python only makes the syscall on a duplicate of Node's open file description.
// Its exit closes that duplicate; Node retains the same flock until close/exit.
// https://developer.apple.com/library/archive/documentation/System/Conceptual/ManPages_iPhoneOS/man2/flock.2.html
const acquire = `import fcntl, sys
try:
    fcntl.flock(3, fcntl.LOCK_EX | fcntl.LOCK_NB)
except BlockingIOError:
    sys.exit(75)
`;
export class RehearsalLock {
  private fd: number | null;
  private path: string;
  constructor(directory: string) {
    this.path = join(directory, "cleanup.lock");
    const fd = openSync(
      this.path,
      constants.O_CREAT | constants.O_RDWR | constants.O_NOFOLLOW,
      0o600,
    );
    this.fd = fd;
    try {
      this.assertHeld();
      const result = spawnSync("python3", ["-I", "-c", acquire], {
        stdio: ["ignore", "pipe", "pipe", fd],
        timeout: 5000,
      });
      assert.equal(
        result.status,
        0,
        result.status === 75
          ? "Another rehearsal/recovery owns cleanup"
          : "Python 3 with fcntl.flock is required; no cleanup authority acquired",
      );
      this.assertHeld();
    } catch (error) {
      this.close();
      throw error;
    }
  }
  assertHeld() {
    assert(this.fd !== null, "Cleanup ownership has been released");
    const held = fstatSync(this.fd),
      named = lstatSync(this.path);
    assert(
      held.isFile() &&
        !named.isSymbolicLink() &&
        held.dev === named.dev &&
        held.ino === named.ino &&
        (held.mode & 0o077) === 0 &&
        held.uid === process.getuid?.(),
      "Owned lock inode changed; refuse cleanup",
    );
  }
  close() {
    if (this.fd !== null) {
      const fd = this.fd;
      this.fd = null;
      closeSync(fd); // Never unlink: a later owner can hold this same permanent inode.
    }
  }
}
