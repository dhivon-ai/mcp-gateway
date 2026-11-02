/** Disposable Compose deployment/restore rehearsal. Accepts no production DSN, project or key. */
import assert from "node:assert/strict";
import { createServer as httpServer, request as httpRequest } from "node:http";
import { spawn, spawnSync, type ChildProcess } from "node:child_process";
import { createHash, randomBytes, randomUUID } from "node:crypto";
import {
  closeSync,
  copyFileSync,
  mkdirSync,
  mkdtempSync,
  openSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { createServer } from "node:net";
import { createInterface } from "node:readline";
import { setTimeout as pause } from "node:timers/promises";
import { join, resolve } from "node:path";
import { verifyImageNotices } from "./check-notices.ts";

const cache = process.env.DHIVON_MCP_GATEWAY_BUILD_CACHE;
assert(cache, "Set the owned DHIVON_MCP_GATEWAY_BUILD_CACHE");
const directory = mkdtempSync(join(realpathSync(cache), "release-rehearsal-"));
const root = resolve(import.meta.dirname, "..");
const image = "dhivon-mcp-gateway-server:0.1.0";
verifyImageNotices(image);
const id = randomUUID().replaceAll("-", "");
const projects = [`gw-rehearsal-${id}`, `gw-restore-${id}`];
const key = join(directory, "vault-keys.json");
const wrongKey = join(directory, "wrong-keys.json");
const password = randomBytes(32).toString("hex");
const secretValue = randomBytes(32).toString("hex");
const origin = "https://gateway.invalid";
const uid = process.getuid?.();
const gid = process.getgid?.();
assert(
  uid !== undefined && gid !== undefined && uid !== 0,
  "Run as an ordinary Unix user",
);
const logFile = openSync(join(directory, "commands.log"), "wx", 0o600);
function run(args: string[], input?: string | number, output?: number): Buffer {
  const r = spawnSync("docker", args, {
    input: typeof input === "string" ? input : undefined,
    stdio: [
      typeof input === "number" ? input : "pipe",
      output ?? "pipe",
      "pipe",
    ],
    timeout: 180_000,
    maxBuffer: 32 * 1024 * 1024,
  });
  if (r.stderr) writeFileSync(logFile, r.stderr);
  assert.equal(
    r.status,
    0,
    "Owned Docker operation failed; inspect private commands.log",
  );
  return Buffer.isBuffer(r.stdout) ? r.stdout : Buffer.alloc(0);
}
const environment = projects.map((_, i) =>
  join(directory, `deployment-${i}.env`),
);
function env(i: number, port: number, keyFile = key) {
  writeFileSync(
    environment[i]!,
    `POSTGRES_PASSWORD=${password}\nDHIVON_MCP_GATEWAY_PUBLIC_URL=${origin}\nDHIVON_MCP_GATEWAY_KEY_FILE=${keyFile}\nDHIVON_MCP_GATEWAY_IMAGE=${image}\nDHIVON_MCP_GATEWAY_PORT=${port}\nDHIVON_MCP_GATEWAY_UID=${uid}\nDHIVON_MCP_GATEWAY_GID=${gid}\n`,
    { mode: 0o600 },
  );
}
function compose(
  i: number,
  args: string[],
  input?: string | number,
  output?: number,
) {
  return run(
    [
      "compose",
      "--env-file",
      environment[i]!,
      "-p",
      projects[i]!,
      "-f",
      join(root, "deploy/compose.yaml"),
      ...args,
    ],
    input,
    output,
  );
}
async function port(): Promise<number> {
  const server = createServer();
  await new Promise<void>((done, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", done);
  });
  const address = server.address();
  assert(address && typeof address === "object");
  await new Promise<void>((done, reject) =>
    server.close((error) => (error ? reject(error) : done())),
  );
  return address.port;
}
function object(value: unknown): Record<string, unknown> {
  assert(
    value && typeof value === "object" && !Array.isArray(value),
    "Expected fixture object",
  );
  return value as Record<string, unknown>;
}
function text(value: unknown): string {
  assert.equal(typeof value, "string");
  return value as string;
}
function number(value: unknown): number {
  assert.equal(typeof value, "number");
  return value as number;
}
// Use native HTTP to preserve the configured public Host on the private proxy hop.
// Node fetch may rewrite Host; this fixture never disables the service's exact-host guard.
async function wire(
  url: string,
  method = "GET",
  token?: string,
  body?: unknown,
): Promise<{ status: number; bytes: Buffer }> {
  return new Promise((done, reject) => {
    const content = body ? JSON.stringify(body) : undefined;
    const request = httpRequest(
      url,
      {
        method,
        agent: false,
        headers: {
          host: "gateway.invalid",
          ...(token ? { authorization: `Bearer ${token}` } : {}),
          ...(content
            ? {
                "content-type": "application/json",
                "content-length": Buffer.byteLength(content),
              }
            : {}),
        },
      },
      (response) => {
        const chunks: Buffer[] = [];
        let size = 0;
        response.on("data", (chunk: Buffer) => {
          size += chunk.length;
          if (size > 2 * 1024 * 1024)
            request.destroy(new Error("Fixture response too large"));
          else chunks.push(chunk);
        });
        response.on("error", reject);
        response.on("end", () =>
          done({
            status: response.statusCode ?? 0,
            bytes: Buffer.concat(chunks),
          }),
        );
      },
    );
    const timer = setTimeout(
      () => request.destroy(new Error("Fixture HTTP deadline")),
      10_000,
    );
    request.on("close", () => clearTimeout(timer));
    request.on("error", reject);
    request.end(content);
  });
}
async function request(
  base: string,
  path: string,
  token?: string,
  method = "GET",
  body?: unknown,
  expected = 200,
): Promise<unknown> {
  const response = await wire(`${base}${path}`, method, token, body);
  assert.equal(response.status, expected, `Fixture response ${method} ${path}`);
  return response.status === 204
    ? null
    : (JSON.parse(response.bytes.toString()) as unknown);
}
async function login(base: string, email: string): Promise<string> {
  return text(
    object(
      await request(base, "/api/endpoint/login", undefined, "POST", {
        email: `${email}@example.test`,
        password,
      }),
    ).token,
  );
}
async function snapshot(base: string, token: string) {
  return object(await request(base, "/api/config", token));
}
async function revision(base: string, token: string) {
  return number((await snapshot(base, token)).revision);
}
function mcps(snapshot: Record<string, unknown>): Record<string, unknown>[] {
  assert(Array.isArray(snapshot.mcps));
  return snapshot.mcps.map(object);
}

async function nativeJourney(base: string, admin: string) {
  const archive = process.env.DHIVON_MCP_GATEWAY_TEST_CLI;
  const fixture = process.env.DHIVON_MCP_GATEWAY_FIXTURE_BIN;
  if (!archive && !fixture) return;
  assert(archive && fixture, "Supply both extracted CLI and built MCP fixture");
  const native = join(directory, "native");
  mkdirSync(native, { mode: 0o700 });
  const statePath = realpathSync(native);
  const cli = join(directory, "dhivon-mcp-gateway");
  copyFileSync(archive, cli); // immutable creating binary retained until Keychain cleanup
  const localMcp = randomUUID();
  await request(base, "/api/admin/mcps", admin, "POST", {
    expectedRevision: await revision(base, admin),
    definition: {
      id: localMcp,
      name: "Packaged STDIO fixture",
      description: "Container to Mac to agent",
      active: true,
      transport: {
        kind: "stdio",
        command: {
          program: realpathSync(fixture),
          args: [],
          env: {},
          cwd: null,
        },
      },
      install: { program: "/usr/bin/true", args: [], env: {}, cwd: null },
      auth: { kind: "none" },
      toolPolicy: { mode: "allowlist", tools: ["echo"] },
    },
  });
  await request(base, "/api/admin/assignments", admin, "POST", {
    expectedRevision: await revision(base, admin),
    mcpId: localMcp,
    target: { kind: "all" },
  });
  // Fixture-only loopback proxy preserves the public Host on the internal hop.
  const proxy = httpServer((incoming, outgoing) => {
    const upstream = httpRequest(
      `${base}${incoming.url ?? "/"}`,
      {
        method: incoming.method,
        agent: false,
        headers: { ...incoming.headers, host: "gateway.invalid" },
      },
      (response) => {
        outgoing.writeHead(response.statusCode ?? 502, response.headers);
        response.pipe(outgoing);
      },
    );
    upstream.setTimeout(10_000, () => upstream.destroy());
    upstream.on("error", () => {
      outgoing.writeHead(502);
      outgoing.end();
    });
    incoming.pipe(upstream);
  });
  await new Promise<void>((done) => proxy.listen(0, "127.0.0.1", done));
  const address = proxy.address();
  assert(address && typeof address === "object");
  const uiPort = await port();
  const uiBase = `http://127.0.0.1:${uiPort}`;
  const common = ["--allow-fixture-http", "--fixture-state-path", statePath];
  const children: ChildProcess[] = [];
  function child(args: string[], protocol = false) {
    const process = spawn(cli, args, {
      stdio: [
        protocol ? "pipe" : "ignore",
        protocol ? "pipe" : logFile,
        logFile,
      ],
    });
    children.push(process);
    return process;
  }
  async function stop(process: ChildProcess) {
    if (process.exitCode !== null || process.signalCode !== null) return;
    process.kill("SIGTERM");
    await Promise.race([
      new Promise<void>((done, reject) => {
        process.once("exit", () => done());
        process.once("error", reject);
      }),
      pause(6000).then(() => {
        throw new Error("Owned native child did not exit; recovery retained");
      }),
    ]);
  }
  try {
    child([
      "ui",
      "--server",
      `http://127.0.0.1:${address.port}`,
      ...common,
      "--port",
      String(uiPort),
      "--no-open",
    ]);
    let cookie = "";
    for (let i = 0; i < 100; i++) {
      try {
        const page = await fetch(uiBase, { signal: AbortSignal.timeout(1000) });
        assert.equal(page.status, 200);
        cookie = page.headers.get("set-cookie")?.split(";")[0] ?? "";
        assert((await page.text()).includes('data-gateway-mode="local"'));
        break;
      } catch {
        await pause(50);
      }
    }
    assert(cookie, "Packaged local UI is ready");
    async function local(
      path: string,
      method = "GET",
      body?: unknown,
      expected = 200,
    ) {
      const response = await fetch(`${uiBase}${path}`, {
        method,
        signal: AbortSignal.timeout(90_000),
        headers: {
          cookie,
          origin: uiBase,
          "x-csrf-token": csrf,
          ...(generation ? { "x-endpoint-generation": generation } : {}),
          "content-type": "application/json",
        },
        body: body ? JSON.stringify(body) : undefined,
      });
      assert.equal(
        response.status,
        expected,
        `Packaged native ${method} ${path}`,
      );
      return object(await response.json());
    }
    let csrf = "",
      generation = "";
    csrf = text((await local("/api/context")).csrfToken);
    const logged = await local("/api/auth/login", "POST", {
      email: "alice@example.test",
      password,
    });
    generation = text(object(logged.identity).generation);
    let catalog = await local("/api/catalog");
    const install = await local(
      `/api/mcps/${localMcp}/install`,
      "POST",
      {
        expectedGeneration: generation,
        input: {
          expectedRevision: number(catalog.revision),
          enableAfterInstall: true,
          secretId: null,
          rerun: false,
        },
      },
      202,
    );
    for (let i = 0; i < 100; i++) {
      const progress = await local(`/api/installations/${text(install.id)}`);
      if (progress.state !== "running") {
        assert.equal(progress.state, "enabled");
        break;
      }
      await pause(50);
    }
    catalog = await local("/api/catalog");
    assert(Array.isArray(catalog.mcps));
    assert(
      catalog.mcps
        .map(object)
        .some((mcp) => mcp.id === localMcp && mcp.enabled === true),
    );
    const gateway = child(["mcp", ...common], true);
    assert(gateway.stdin && gateway.stdout);
    const protocolInput = gateway.stdin;
    const lines = createInterface({ input: gateway.stdout });
    const pending = new Map<
      number,
      {
        done: (value: Record<string, unknown>) => void;
        fail: (error: Error) => void;
      }
    >();
    lines.on("line", (line) => {
      try {
        const reply = object(JSON.parse(line) as unknown);
        if (typeof reply.id === "number") pending.get(reply.id)?.done(reply);
      } catch {
        for (const item of pending.values())
          item.fail(new Error("Non-protocol stdout"));
      }
    });
    let id = 0;
    async function rpc(method: string, params: unknown) {
      const requestId = ++id;
      return new Promise<Record<string, unknown>>((done, fail) => {
        const timer = setTimeout(() => {
          pending.delete(requestId);
          fail(new Error("Packaged agent deadline"));
        }, 40_000);
        pending.set(requestId, {
          done: (reply) => {
            clearTimeout(timer);
            pending.delete(requestId);
            done(reply);
          },
          fail,
        });
        protocolInput.write(
          `${JSON.stringify({ jsonrpc: "2.0", id: requestId, method, params })}\n`,
        );
      });
    }
    const initialized = object(
      (
        await rpc("initialize", {
          protocolVersion: "2025-11-25",
          capabilities: {},
          clientInfo: { name: "release-one-entry-rehearsal", version: "1" },
        })
      ).result,
    );
    assert.equal(initialized.protocolVersion, "2025-11-25");
    protocolInput.write(
      `${JSON.stringify({ jsonrpc: "2.0", method: "notifications/initialized" })}\n`,
    );
    const listed = object((await rpc("tools/list", {})).result);
    assert(Array.isArray(listed.tools));
    const toolName = `gw_${createHash("sha256").update(localMcp).digest("hex").slice(0, 32)}_${createHash("sha256").update("echo").digest("hex")}`;
    const tool = listed.tools
      .map(object)
      .find((tool) => tool.name === toolName);
    assert(tool, "The exact assigned packaged STDIO tool");
    const called = object(
      (
        await rpc("tools/call", {
          name: text(tool.name),
          arguments: { message: "release-rehearsal" },
        })
      ).result,
    );
    assert.equal(object(called.structuredContent).message, "release-rehearsal");
    catalog = await local("/api/catalog");
    await local(`/api/settings/${localMcp}`, "PUT", {
      expectedGeneration: generation,
      input: {
        expectedRevision: number(catalog.revision),
        enabled: false,
        secretId: null,
      },
    });
    const denied = await rpc("tools/call", {
      name: text(tool.name),
      arguments: { message: "denied" },
    });
    assert(denied.error, "Confirmed disable blocks subsequent dispatch");
    await stop(gateway);
    lines.close();
    console.log(
      "PASS: container administrator→packaged Mac login/install/enable→one STDIO tool→confirmed disable denial; actual Keychain.",
    );
  } finally {
    for (const process of children.reverse()) await stop(process);
    const logout = spawn(cli, ["logout", ...common], {
      stdio: ["ignore", logFile, logFile],
    });
    assert.equal(
      await new Promise<number | null>((done, reject) => {
        logout.once("exit", done);
        logout.once("error", reject);
      }),
      0,
      "Creating archive binary logout",
    );
    const metadata = object(
      JSON.parse(
        readFileSync(join(statePath, "identity.json"), "utf8"),
      ) as unknown,
    );
    assert.equal(metadata.active, null);
    assert.deepEqual(metadata.credentialAccounts, []);
    assert.deepEqual(metadata.pendingCleanup, []);
    await new Promise<void>((done, reject) =>
      proxy.close((error) => (error ? reject(error) : done())),
    );
  }
}

const ports = [await port(), await port()];
const bases = ports.map((value) => `http://127.0.0.1:${value}`);
for (let i = 0; i < 2; i++) env(i, ports[i]!);
let succeeded = false;
try {
  for (const name of ["vault-keys.json", "wrong-keys.json"]) {
    run([
      "run",
      "--rm",
      "--user",
      `${uid}:${gid}`,
      "--mount",
      `type=bind,source=${directory},target=/keys`,
      image,
      "generate-vault-key",
      "--output",
      `/keys/${name}`,
    ]);
  }
  console.log("Rehearsal: Fresh deployment");
  compose(0, ["up", "-d", "--wait", "postgres"]);
  compose(
    0,
    [
      "run",
      "--rm",
      "--no-deps",
      "-T",
      "dhivon",
      "bootstrap-admin",
      "--email",
      "admin@example.test",
      "--name",
      "Administrator",
      "--password-stdin",
    ],
    password,
  );
  compose(0, ["up", "-d", "--wait", "dhivon"]);
  const base = bases[0]!;
  await request(base, "/healthz");
  const html = await wire(base);
  assert.equal(html.status, 200);
  const page = html.bytes.toString();
  const script = page.match(/src="(\/assets\/[^"]+\.js)"/u)?.[1];
  assert(script, "Compiled administrator assets");
  const asset = await wire(`${base}${script}`);
  assert.equal(asset.status, 200);
  assert(asset.bytes.length > 1000);
  const admin = await login(base, "admin");
  for (const email of ["alice", "bob"]) {
    await request(base, "/api/admin/users", admin, "POST", {
      expectedRevision: await revision(base, admin),
      name: email,
      email: `${email}@example.test`,
      password,
      role: "user",
    });
  }
  for (const name of ["engineering", "security"]) {
    await request(base, "/api/admin/groups", admin, "POST", {
      expectedRevision: await revision(base, admin),
      name,
      memberIds: [],
    });
  }
  const alice = await login(base, "alice");
  const bob = await login(base, "bob");
  await request(base, "/api/admin/users", alice, "GET", undefined, 403);
  const mcp = randomUUID();
  await request(base, "/api/admin/mcps", admin, "POST", {
    expectedRevision: await revision(base, admin),
    definition: {
      id: mcp,
      name: "Restore fixture",
      description: "Owned backup rehearsal",
      active: true,
      transport: { kind: "http", url: "https://fixture.invalid/mcp" },
      install: null,
      auth: { kind: "bearer" },
      toolPolicy: { mode: "allowlist", tools: ["echo"] },
    },
  });
  assert.equal(mcps(await snapshot(base, alice)).length, 0);
  await request(base, "/api/admin/assignments", admin, "POST", {
    expectedRevision: await revision(base, admin),
    mcpId: mcp,
    target: { kind: "all" },
  });
  assert.equal(mcps(await snapshot(base, alice))[0]?.enabled, false);
  const secret = object(
    await request(base, "/api/secrets", alice, "POST", {
      key: "release-fixture",
      value: secretValue,
      expectedVersion: null,
    }),
  );
  assert(!JSON.stringify(secret).includes(secretValue));
  assert.deepEqual(await request(base, "/api/secrets", bob), []);
  await request(base, `/api/settings/${mcp}`, alice, "PUT", {
    expectedRevision: await revision(base, alice),
    enabled: true,
    secretId: text(secret.id),
  });
  await nativeJourney(base, admin);
  const before = await snapshot(base, alice);
  const installationId = randomUUID();
  async function credential(target: string, token: string, expected = 200) {
    return object(
      await request(
        target,
        `/api/endpoint/credentials/${mcp}`,
        token,
        "POST",
        { expectedRevision: await revision(target, token), installationId },
        expected,
      ),
    );
  }
  assert.equal((await credential(base, alice)).token, secretValue);
  const db = compose(0, [
    "exec",
    "-T",
    "postgres",
    "psql",
    "-U",
    "dhivon",
    "-d",
    "dhivon",
    "-At",
    "-c",
    "SELECT encode(ciphertext,'hex') FROM user_secrets",
  ]);
  assert(!db.includes(Buffer.from(secretValue)));
  console.log("Rehearsal: Restart durability");
  compose(0, ["restart", "dhivon"]);
  compose(0, ["up", "-d", "--wait", "dhivon"]);
  const restarted = await snapshot(base, alice);
  assert.equal(restarted.deploymentId, before.deploymentId);
  assert.equal(restarted.revision, before.revision);
  assert.equal(
    mcps(restarted).find((item) => object(item.definition).id === mcp)?.enabled,
    true,
  );
  assert.equal((await credential(base, alice)).token, secretValue);
  console.log("Rehearsal: Database backup and fresh restore");
  const dump = join(directory, "database.dump");
  const output = openSync(dump, "wx", 0o600);
  try {
    compose(
      0,
      [
        "exec",
        "-T",
        "postgres",
        "pg_dump",
        "-U",
        "dhivon",
        "-d",
        "dhivon",
        "-Fc",
      ],
      undefined,
      output,
    );
  } finally {
    closeSync(output);
  }
  assert(!readFileSync(dump).includes(Buffer.from(secretValue)));
  compose(1, ["up", "-d", "--wait", "postgres"]);
  const input = openSync(dump, "r");
  try {
    compose(
      1,
      [
        "exec",
        "-T",
        "postgres",
        "pg_restore",
        "-U",
        "dhivon",
        "-d",
        "dhivon",
        "--no-owner",
        "--exit-on-error",
      ],
      input,
    );
  } finally {
    closeSync(input);
  }
  compose(1, [
    "exec",
    "-T",
    "postgres",
    "psql",
    "-U",
    "dhivon",
    "-d",
    "dhivon",
    "-v",
    "ON_ERROR_STOP=1",
    "-c",
    "DELETE FROM sessions;",
  ]);
  compose(1, ["up", "-d", "--wait", "dhivon"]);
  const restoredBase = bases[1]!;
  await request(restoredBase, "/api/config", alice, "GET", undefined, 401);
  const restored = await login(restoredBase, "alice");
  const after = await snapshot(restoredBase, restored);
  assert.equal(after.deploymentId, before.deploymentId);
  assert.equal(after.revision, before.revision);
  assert.equal(
    mcps(after).find((item) => object(item.definition).id === mcp)?.enabled,
    true,
  );
  assert.equal((await credential(restoredBase, restored)).token, secretValue);
  // Matching database alone is insufficient; the independent wrong key must fail closed.
  console.log("Rehearsal: Wrong-key denial and matching-key recovery");
  env(1, ports[1]!, wrongKey);
  compose(1, ["up", "-d", "--wait", "--force-recreate", "dhivon"]);
  await credential(restoredBase, restored, 503);
  env(1, ports[1]!);
  compose(1, ["up", "-d", "--wait", "--force-recreate", "dhivon"]);
  assert.equal((await credential(restoredBase, restored)).token, secretValue);
  const logs = Buffer.concat(
    projects.map((_, i) => compose(i, ["logs", "--no-color", "dhivon"])),
  );
  for (const value of [password, secretValue, alice, bob, admin, restored])
    assert(
      !logs.includes(Buffer.from(value)),
      "Captured service logs are redacted",
    );
  console.log(
    "PASS: fresh Compose/migrations/health/embedded assets/default deny/roles/vault; restart; pg_dump→fresh pg_restore; independent wrong-key denial and matching-key recovery; redacted logs.",
  );
  succeeded = true;
} finally {
  // Names and paths are generated above; never accept an existing deployment/project/volume.
  for (let i = 1; i >= 0; i--)
    compose(i, ["down", "--volumes", "--remove-orphans"]);
  closeSync(logFile);
  if (succeeded) rmSync(directory, { recursive: true });
  else console.error(`Owned recovery evidence retained: ${directory}`);
}
