import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { test } from "node:test";
import { setTimeout as pause } from "node:timers/promises";
const cache = process.env.DHIVON_MCP_GATEWAY_BUILD_CACHE;
assert(
  cache && process.env.DHIVON_MCP_GATEWAY_TEST_CLI,
  "Requires owned cache and actual extracted Mac CLI",
);
const evidence: unknown[] = [];
function read(path: string) {
  return JSON.parse(readFileSync(path, "utf8"));
}
function docker(args: string[]) {
  const result = spawnSync("docker", args, {
    encoding: "utf8",
    timeout: 15_000,
  });
  assert.equal(result.status, 0);
  return result.stdout.trim();
}
function absent(directory: string) {
  const j = read(join(directory, "ownership.json"));
  for (const [kind, args] of [
    ["container", ["ps", "-aq"]],
    ["network", ["network", "ls", "-q"]],
    ["volume", ["volume", "ls", "-q"]],
  ] as const) {
    assert.equal(
      docker([
        ...args,
        "--filter",
        `label=com.docker.compose.project=${j.project}`,
      ]),
      "",
      `${kind} absent`,
    );
  }
  assert.equal(
    docker(["ps", "-aq", "--filter", `name=^/${j.project}-keygen$`]),
    "",
  );
  for (const child of j.children) {
    if (!child.owner?.identity) continue;
    const current = spawnSync(
      "ps",
      ["-p", String(child.owner.pid), "-o", "lstart=", "-o", "command="],
      { encoding: "utf8", timeout: 2000 },
    );
    assert.notEqual(
      current.stdout.trim(),
      child.owner.identity,
      "Exact owned child is gone",
    );
  }
  const metadata = read(join(j.state, "identity.json"));
  assert.equal(metadata.active, null);
  assert.deepEqual(metadata.credentialAccounts, []);
  assert.deepEqual(metadata.pendingCleanup, []);
  assert(
    existsSync(join(directory, "cleanup.lock")),
    "Permanent lock inode retained after release",
  );
}
async function probe(mode: string, signal?: NodeJS.Signals) {
  const p = spawn(
    process.execPath,
    [join(import.meta.dirname, "rehearsal-cleanup-fixture.ts"), mode],
    { stdio: ["ignore", "pipe", "pipe", "ipc"] },
  );
  let directory = "";
  let stderr = "";
  p.stderr?.on("data", (b) => {
    stderr += b.toString();
  });
  const done = new Promise<number | null>((resolve, reject) => {
    p.once("exit", (code, sig) => {
      if (sig) reject(new Error(`Unsupervised exit ${sig}: ${stderr}`));
      else resolve(code);
    });
    p.once("error", reject);
  });
  const timer = setTimeout(() => p.kill("SIGTERM"), 150_000);
  p.on("message", (message: unknown) => {
    const m = message as { directory?: string; ready?: string };
    if (m.directory) directory = m.directory;
    if (m.ready && signal) {
      const rejected = spawnSync(
        process.execPath,
        [join(import.meta.dirname, "recover-https-oauth.ts"), directory],
        { encoding: "utf8", timeout: 5000, env: process.env },
      );
      assert.notEqual(rejected.status, 0, "Active supervisor blocks recovery");
      assert.match(
        rejected.stderr,
        /original supervisor is alive|Another rehearsal\/recovery owns cleanup/,
      );
      p.kill(signal);
      setTimeout(() => {
        if (p.exitCode === null) p.kill(signal);
      }, 50);
    }
  });
  const code = await done.finally(() => clearTimeout(timer));
  assert(directory, stderr);
  const result = read(join(directory, "result.json"));
  evidence.push({ mode, signal, code, directory, result });
  writeFileSync(
    join(cache!, "cleanup-test-results.json"),
    JSON.stringify(evidence, null, 2),
    { mode: 0o600 },
  );
  return { directory, code, result };
}

test(
  "real Docker/native rehearsal cleanup",
  { timeout: 600_000 },
  async (t) => {
    // An unrelated process must survive every cleanup and recovery invocation.
    const unrelated = spawn(
      process.execPath,
      ["-e", "setInterval(() => {}, 1000)"],
      { stdio: "ignore" },
    );
    try {
      for (const mode of ["resources", "native"])
        for (const signal of ["SIGINT", "SIGTERM"] as const) {
          await t.test(`${signal} after ${mode}`, async () => {
            const { directory, code, result } = await probe(mode, signal);
            assert.equal(code, signal === "SIGINT" ? 130 : 143);
            assert.equal(result.interrupted, signal);
            assert.equal(result.succeeded, false);
            assert.equal(result.cleanupStatus, "clean");
            absent(directory);
          });
        }
      for (const mode of ["normal", "diagnostics", "failure"])
        await t.test(mode, async () => {
          const { directory, code, result } = await probe(mode);
          assert.equal(code, mode === "normal" ? 0 : 1);
          assert.equal(result.succeeded, mode === "normal");
          assert.equal(result.cleanupStatus, "clean");
          assert.equal(result.cleanup.diagnostics.ok, mode !== "diagnostics");
          absent(directory);
        });
      for (const mode of ["incomplete", "incomplete-native"])
        await t.test(`${mode}: retained recovery is repeatable`, async () => {
          const { directory, code, result } = await probe(mode);
          assert.equal(code, 1);
          assert.equal(result.cleanupStatus, "recovery_required");
          assert.equal(result.stacksRemoved, false);
          assert.equal(
            result.nativeCredentialsCleared,
            mode !== "incomplete-native",
          );
          assert(existsSync(join(directory, "dhivon-mcp-gateway")));
          assert(
            docker([
              "volume",
              "ls",
              "-q",
              "--filter",
              `label=com.docker.compose.project=${result.project}`,
            ]),
          );
          if (mode === "incomplete-native") {
            const metadata = read(join(directory, "native/identity.json"));
            assert(metadata.active && metadata.credentialAccounts.length > 0);
          }
          const composeFile = join(directory, "compose.yaml");
          const original = readFileSync(composeFile);
          writeFileSync(
            composeFile,
            Buffer.concat([original, Buffer.from("\n# changed input\n")]),
          );
          try {
            const rejected = spawnSync(
              process.execPath,
              [join(import.meta.dirname, "recover-https-oauth.ts"), directory],
              { encoding: "utf8", timeout: 5000, env: process.env },
            );
            assert.notEqual(
              rejected.status,
              0,
              "Changed ownership inputs block destructive recovery",
            );
            assert.match(rejected.stderr, /Owned Compose inputs changed/);
          } finally {
            writeFileSync(composeFile, original);
          }
          for (let i = 0; i < 2; i++) {
            const recover = spawnSync(
              process.execPath,
              [join(import.meta.dirname, "recover-https-oauth.ts"), directory],
              { encoding: "utf8", timeout: 90_000, env: process.env },
            );
            assert.equal(recover.status, 0, recover.stderr);
            absent(directory);
          }
          const recovered = read(join(directory, "result.json"));
          assert.equal(recovered.cleanupStatus, "clean");
          assert.equal(
            recovered.succeeded,
            false,
            "Recovery must not turn the earlier failed run into successful verification",
          );
          assert.equal(recovered.cleanupHistory.length, 3);
          assert.equal(recovered.cleanupHistory[0].status, "recovery_required");
          evidence.push({
            mode: `${mode}-recovered`,
            directory,
            result: recovered,
          });
          writeFileSync(
            join(cache!, "cleanup-test-results.json"),
            JSON.stringify(evidence, null, 2),
            { mode: 0o600 },
          );
        });
      await t.test(
        "original HTTPS entry point: SIGTERM during PostgreSQL startup",
        async () => {
          assert(
            process.env.DHIVON_MCP_GATEWAY_FIXTURE_BIN &&
              process.env.DHIVON_MCP_GATEWAY_TEST_TLS_DIR,
            "Supply fixture and public test certificate directory; no OS trust required for this early stop",
          );
          const p = spawn(
            process.execPath,
            [join(import.meta.dirname, "verify-https-oauth.ts")],
            { stdio: ["ignore", "pipe", "pipe"] },
          );
          let directory = "";
          let output = "";
          p.stdout.on("data", (b) => {
            output += b.toString();
            directory =
              output.match(/Ownership record: (.+)\/ownership.json/)?.[1] ??
              directory;
          });
          p.stderr.on("data", () => {});
          const done = new Promise<number | null>((resolve) =>
            p.once("exit", resolve),
          );
          try {
            let sent = false;
            for (let i = 0; i < 300 && p.exitCode === null; i++) {
              if (directory) {
                const j = read(join(directory, "ownership.json"));
                if (
                  docker([
                    "ps",
                    "-q",
                    "--filter",
                    `label=com.docker.compose.project=${j.project}`,
                    "--filter",
                    "status=running",
                  ])
                ) {
                  p.kill("SIGTERM");
                  sent = true;
                  break;
                }
              }
              await pause(100);
            }
            assert(
              sent,
              "Signal sent after actual generated PostgreSQL started",
            );
            const code = await done;
            assert.equal(code, 143);
            absent(directory);
            const result = read(join(directory, "result.json"));
            assert.equal(result.interrupted, "SIGTERM");
            assert.equal(result.cleanupStatus, "clean");
            evidence.push({
              mode: "actual-https-entry-startup",
              code,
              directory,
              result,
            });
            writeFileSync(
              join(cache!, "cleanup-test-results.json"),
              JSON.stringify(evidence, null, 2),
              { mode: 0o600 },
            );
          } finally {
            if (p.exitCode === null) {
              p.kill("SIGTERM");
              await done;
            }
          }
        },
      );
      assert(unrelated.pid);
      process.kill(unrelated.pid, 0);
    } finally {
      unrelated.kill("SIGKILL");
    }
  },
);
