import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import {
  existsSync,
  lstatSync,
  mkdtempSync,
  readFileSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import { RehearsalLock } from "./rehearsal-lock.ts";
const cache = process.env.DHIVON_MCP_GATEWAY_BUILD_CACHE;
assert(cache && process.env.DHIVON_MCP_GATEWAY_TEST_CLI);
const fixture = join(import.meta.dirname, "rehearsal-ownership-fixture.ts");
const evidence: unknown[] = [];
let recoveredDirectory = "";
function record(value: unknown) {
  evidence.push(value);
  writeFileSync(
    join(cache!, "ownership-test-results.json"),
    JSON.stringify(evidence, null, 2),
    { mode: 0o600 },
  );
}
function child(mode: string, directory?: string) {
  const p = spawn(
    process.execPath,
    [fixture, mode, ...(directory ? [directory] : [])],
    { stdio: ["ignore", "ignore", "pipe", "ipc"] },
  );
  const messages: unknown[] = [];
  let error = "";
  p.stderr?.on("data", (b) => {
    error += b.toString();
  });
  let wake: (() => void) | undefined;
  p.on("message", (message) => {
    messages.push(message);
    wake?.();
  });
  const ended = new Promise<{
    code: number | null;
    signal: NodeJS.Signals | null;
  }>((done, fail) => {
    p.on("error", fail);
    p.on("exit", (code, signal) => {
      done({ code, signal });
      wake?.();
    });
  });
  async function receive() {
    while (!messages.length) {
      assert(
        p.exitCode === null && p.signalCode === null,
        `Probe exited before message: ${error}`,
      );
      await new Promise<void>((done, fail) => {
        const timer = setTimeout(() => {
          wake = undefined;
          fail(new Error(`Probe message deadline: ${error}`));
        }, 10_000);
        wake = () => {
          clearTimeout(timer);
          wake = undefined;
          done();
        };
      });
    }
    return messages.shift() as Record<string, unknown>;
  }
  return { p, receive, ended };
}
function privateLockDirectory() {
  return mkdtempSync(join(cache!, "lock-regression-"));
}
function json(path: string) {
  return JSON.parse(readFileSync(path, "utf8"));
}
function noDocker(project: string) {
  for (const args of [
    ["ps", "-aq"],
    ["network", "ls", "-q"],
    ["volume", "ls", "-q"],
  ]) {
    const r = spawnSync(
      "docker",
      [...args, "--filter", `label=com.docker.compose.project=${project}`],
      { encoding: "utf8", timeout: 10_000 },
    );
    assert.equal(r.status, 0);
    assert.equal(r.stdout.trim(), "");
  }
}

test(
  "child registration failures retain supervision",
  { timeout: 90_000 },
  async (t) => {
    for (const mode of [
      "identity-failure",
      "cleanup-registration-failure",
      "persistence-failure",
      "registration-signal",
      "unresolved-intent",
    ])
      await t.test(mode, async () => {
        const probe = child(mode);
        const message = await probe.receive();
        const exit = await probe.ended;
        assert.equal(typeof message.directory, "string");
        const directory = String(message.directory);
        const proof = json(join(directory, "probe.json"));
        const result = json(join(directory, "result.json"));
        const j = json(join(directory, "ownership.json"));
        record({ directory, exit, proof, result });
        assert.equal(proof.ownedChildAlive, false);
        if (mode === "cleanup-registration-failure")
          assert.equal(
            proof.cleanupChildWasLive,
            true,
            "Diagnostic child was live after failed registration and ignored the initial interrupt",
          );
        if (mode === "unresolved-intent") {
          assert.equal(exit.code, 1);
          assert.equal(result.cleanupStatus, "recovery_required");
          assert.equal(result.cleanup.children.ok, false);
          assert.equal(result.cleanup.native.ok, false);
          assert.equal(result.cleanup.stack.ok, false);
          assert(
            !existsSync(join(directory, "native/identity.json")),
            "Uncertain writer does not authorize native logout",
          );
        } else {
          assert.equal(
            exit.code,
            mode === "registration-signal"
              ? 143
              : mode === "cleanup-registration-failure"
                ? 1
                : 0,
          );
          assert.equal(
            proof.registrationFailed,
            mode !== "registration-signal",
          );
          assert.equal(result.cleanupStatus, "clean");
          assert.equal(proof.phase, "exited");
          const metadata = json(join(directory, "native/identity.json"));
          assert.equal(metadata.active, null);
          assert.deepEqual(metadata.credentialAccounts, []);
          assert.deepEqual(metadata.pendingCleanup, []);
        }
        if (mode === "identity-failure") recoveredDirectory = directory;
        noDocker(j.project);
        const lock = new RehearsalLock(directory);
        lock.close();
      });
  },
);

test(
  "recovery ownership is exclusive across races and crashes",
  { timeout: 45_000 },
  async (t) => {
    await t.test("simultaneous stale-lock contenders", async () => {
      const directory = privateLockDirectory();
      writeFileSync(
        join(directory, "cleanup.lock"),
        JSON.stringify({ pid: 1, identity: "stale-fixture-owner" }),
        { mode: 0o600 },
      );
      const inode = lstatSync(join(directory, "cleanup.lock")).ino;
      const a = child("lock", directory),
        b = child("lock", directory);
      try {
        const initial = await Promise.all([a.receive(), b.receive()]);
        assert.equal(initial[0].observed, initial[1].observed);
        a.p.send("acquire");
        b.p.send("acquire");
        const outcome = await Promise.all([a.receive(), b.receive()]);
        assert.equal(outcome.filter((m) => m.acquired).length, 1);
        assert.equal(outcome.filter((m) => m.rejected).length, 1);
        assert.equal(
          lstatSync(join(directory, "cleanup.lock")).ino,
          inode,
          "No stale inode was removed",
        );
        record({
          case: "simultaneous stale contenders",
          directory,
          outcome,
          permanentInode: inode,
        });
      } finally {
        a.p.send("exit");
        b.p.send("exit");
        await Promise.all([a.ended, b.ended]);
      }
    });
    for (const mode of ["lock-partial", "lock"])
      await t.test(`forced exit during ${mode}`, async () => {
        const directory = privateLockDirectory();
        const a = child(mode, directory);
        await a.receive();
        if (mode === "lock") {
          a.p.send("acquire");
          assert.equal((await a.receive()).acquired, true);
        }
        const inode = lstatSync(join(directory, "cleanup.lock")).ino;
        a.p.kill("SIGKILL");
        assert.equal((await a.ended).signal, "SIGKILL");
        const next = new RehearsalLock(directory);
        next.close();
        assert.equal(lstatSync(join(directory, "cleanup.lock")).ino, inode);
        record({ case: `crash ${mode}`, directory, reacquired: true, inode });
      });
    await t.test("old release cannot affect a successor", async () => {
      const directory = privateLockDirectory();
      const a = child("lock", directory),
        b = child("lock", directory);
      try {
        await a.receive();
        await b.receive();
        a.p.send("acquire");
        assert.equal((await a.receive()).acquired, true);
        a.p.send("release");
        await a.receive();
        b.p.send("acquire");
        assert.equal((await b.receive()).acquired, true);
        a.p.send("release");
        await a.receive();
        assert.throws(() => new RehearsalLock(directory), /Another/);
        record({
          case: "old release with live successor",
          directory,
          successorStillExclusive: true,
        });
      } finally {
        a.p.send("exit");
        b.p.send("exit");
        await Promise.all([a.ended, b.ended]);
      }
    });
    await t.test(
      "Rehearsal recovery serializes and reads the latest owned journal",
      async () => {
        assert(recoveredDirectory);
        const a = child("recover-hold", recoveredDirectory);
        await a.receive();
        try {
          const contender = child("recover-hold", recoveredDirectory);
          assert.equal(
            (await contender.ended).code,
            1,
            "Concurrent full recovery refuses ownership",
          );
          a.p.send("stamp");
          await a.receive();
        } finally {
          a.p.send("exit");
          await a.ended;
        }
        const successor = child("recover-hold", recoveredDirectory);
        try {
          assert.equal(
            (await successor.receive()).marker,
            "latest-owned-snapshot",
          );
        } finally {
          successor.p.send("exit");
          await successor.ended;
        }
        record({
          case: "full recovery journal freshness",
          directory: recoveredDirectory,
          latestSnapshotPreserved: true,
        });
      },
    );
    await t.test(
      "missing Python cannot grant ownership or strand the lock",
      () => {
        const directory = privateLockDirectory();
        const previous = process.env.PATH;
        try {
          process.env.PATH = directory;
          assert.throws(
            () => new RehearsalLock(directory),
            /Python 3.*no cleanup authority/,
          );
        } finally {
          process.env.PATH = previous;
        }
        const next = new RehearsalLock(directory);
        next.close();
        record({
          case: "missing Python",
          directory,
          failedClosedAndRetryable: true,
        });
      },
    );
    await t.test("malformed lock bytes cannot strand recovery", () => {
      const directory = privateLockDirectory();
      writeFileSync(join(directory, "cleanup.lock"), "{partial-owner", {
        mode: 0o600,
      });
      const first = new RehearsalLock(directory);
      assert.throws(() => new RehearsalLock(directory), /Another/);
      first.close();
      const next = new RehearsalLock(directory);
      next.close();
      record({ case: "partial bytes", directory, reacquired: true });
    });
  },
);
