/** Process regressions use private generated fixture scopes only. */
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { randomUUID } from "node:crypto";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  realpathSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, join } from "node:path";
import { Rehearsal } from "./rehearsal-lifecycle.ts";
import { RehearsalLock } from "./rehearsal-lock.ts";
const cache = process.env.DHIVON_MCP_GATEWAY_BUILD_CACHE;
const mode = process.argv[2];
assert(cache);
if (mode === "recover-hold") {
  const r = Rehearsal.recover(cache, process.argv[3]);
  process.on("message", (message) => {
    if (message === "stamp") {
      r.journal.details = {
        ...r.journal.details,
        probeMarker: "latest-owned-snapshot",
      };
      r.save();
      process.send?.({ stamped: true });
    } else if (message === "exit") {
      r.close();
      process.disconnect?.();
    }
  });
  process.send?.({
    acquired: true,
    marker: r.journal.details?.probeMarker ?? null,
  });
} else if (mode === "lock" || mode === "lock-partial") {
  const directory = realpathSync(process.argv[3]);
  assert.equal(dirname(directory), realpathSync(cache));
  assert(/^lock-regression-[a-zA-Z0-9]+$/.test(basename(directory)));
  let lock: RehearsalLock | undefined;
  if (mode === "lock-partial")
    writeFileSync(join(directory, "cleanup.lock"), "", { mode: 0o600 });
  process.on("message", (message) => {
    if (message === "acquire") {
      try {
        lock = new RehearsalLock(directory);
        process.send?.({ acquired: true });
      } catch {
        process.send?.({ rejected: true });
      }
    } else if (message === "release") {
      lock?.close();
      process.send?.({ released: true });
    } else if (message === "exit") {
      lock?.close();
      process.disconnect?.();
    }
  });
  process.send?.({
    ready: true,
    observed: existsSync(join(directory, "cleanup.lock"))
      ? readFileSync(join(directory, "cleanup.lock"), "utf8")
      : null,
  });
} else {
  const cli = process.env.DHIVON_MCP_GATEWAY_TEST_CLI;
  assert(
    cli &&
      [
        "identity-failure",
        "cleanup-registration-failure",
        "persistence-failure",
        "registration-signal",
        "unresolved-intent",
      ].includes(mode),
  );
  const r = Rehearsal.create(cache, cli);
  r.supervise();
  const env = `POSTGRES_PASSWORD=controlled-ownership-probe\nDHIVON_MCP_GATEWAY_PUBLIC_URL=https://localhost:59991\nDHIVON_MCP_GATEWAY_KEY_FILE=${join(r.directory, "vault-keys.json")}\nDHIVON_MCP_GATEWAY_IMAGE=dhivon-mcp-gateway-server:0.1.0\nDHIVON_MCP_GATEWAY_PORT=59990\n`;
  writeFileSync(join(r.directory, "deployment.env"), env, { mode: 0o600 });
  copyFileSync(
    join(import.meta.dirname, "../deploy/compose.yaml"),
    join(r.directory, "compose.yaml"),
  );
  writeFileSync(join(r.directory, "tls.yaml"), "{}", { mode: 0o600 });
  r.seal(["deployment.env", "compose.yaml", "tls.yaml"]);
  const idle = join(r.directory, "owned-registration-child.ts");
  writeFileSync(idle, "setInterval(() => {}, 1000);");
  const originalPath = process.env.PATH;
  const save = r.save.bind(r);
  let registrationFailed = false;
  let cleanupChildWasLive = false;
  let clean = false;
  try {
    if (mode === "cleanup-registration-failure") {
      const ready = join(r.directory, "child-ready");
      writeFileSync(
        idle,
        `import {writeFileSync} from "node:fs"; process.on("SIGINT",()=>{}); writeFileSync(${JSON.stringify(ready)},"ready"); setInterval(()=>{},1000);`,
      );
      const bin = join(r.directory, "probe-bin");
      mkdirSync(bin, { mode: 0o700 });
      writeFileSync(
        join(bin, "ps"),
        '#!/bin/sh\nwhile [ ! -f "$DHIVON_MCP_GATEWAY_REVIEW_READY" ]; do /bin/sleep 0.01; done\nexit 2\n',
        { mode: 0o700 },
      );
      const compose = r.compose.bind(r);
      r.compose = (args, options) => {
        if (args[0] !== "logs") return compose(args, options);
        process.env.PATH = `${bin}:${originalPath}`;
        process.env.DHIVON_MCP_GATEWAY_REVIEW_READY = ready;
        try {
          r.child(process.execPath, [idle], false);
          throw new Error("Expected registration failure");
        } catch (error) {
          registrationFailed = true;
          const captured = r.journal.children[0].owner;
          cleanupChildWasLive =
            existsSync(ready) &&
            !!captured &&
            spawnSync(
              "/bin/ps",
              ["-p", String(captured.pid), "-o", "command="],
              { encoding: "utf8" },
            ).stdout.includes(idle);
          throw error;
        } finally {
          process.env.PATH = originalPath;
          delete process.env.DHIVON_MCP_GATEWAY_REVIEW_READY;
        }
      };
      r.journal.workflowSucceeded = true;
    } else if (mode === "unresolved-intent") {
      // A controlled intent with no spawn attempt models a crash-ambiguous disk record.
      r.journal.children.push({
        id: randomUUID(),
        binary: process.execPath,
        args: [idle],
        owner: null,
        phase: "intent",
      });
      r.save();
    } else {
      if (mode === "persistence-failure") {
        let count = 0;
        r.save = () => {
          if (++count === 2)
            throw new Error("Controlled post-spawn persistence failure");
          save();
        };
      } else {
        const bin = join(r.directory, "probe-bin");
        mkdirSync(bin, { mode: 0o700 });
        const fault =
          mode === "identity-failure" ? "exit 2" : '/bin/kill -TERM "$PPID"';
        writeFileSync(
          join(bin, "ps"),
          `#!/bin/sh\nif [ "$2" != "$PPID" ]; then ${fault}; fi\nexec /bin/ps "$@"\n`,
          { mode: 0o700 },
        );
        process.env.PATH = `${bin}:${originalPath}`;
      }
      try {
        r.child(process.execPath, [idle]);
      } catch {
        registrationFailed = true;
      }
      process.env.PATH = originalPath;
      r.save = save;
      if (mode === "registration-signal") await r.wait(5000);
    }
  } catch {
    /* Supervised signal interrupts the wait. */
  } finally {
    process.env.PATH = originalPath;
    r.save = save;
    try {
      clean = await r.cleanup();
    } finally {
      r.close();
    }
  }
  const c = r.journal.children[0];
  const current = c.owner
    ? spawnSync("/bin/ps", ["-p", String(c.owner.pid), "-o", "command="], {
        encoding: "utf8",
      }).stdout
    : "";
  writeFileSync(
    join(r.directory, "probe.json"),
    JSON.stringify({
      mode,
      registrationFailed,
      cleanupChildWasLive,
      clean,
      childPid: c.owner?.pid,
      ownedChildAlive: current.includes(idle),
      phase: c.phase,
    }),
    { mode: 0o600 },
  );
  process.send?.({ directory: r.directory });
  process.disconnect?.();
  process.exitCode =
    mode === "registration-signal"
      ? 143
      : clean && r.journal.cleanup.diagnostics?.ok
        ? 0
        : 1;
}
