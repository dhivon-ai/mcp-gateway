/** Disposable real-Postgres browser fixture. No production DSN or user account is accepted. */
import { spawn, spawnSync, type ChildProcess } from "node:child_process";
import { randomUUID } from "node:crypto";
import { mkdirSync, openSync, closeSync, rmSync, writeFileSync } from "node:fs";
import { resolve, join } from "node:path";
import { setTimeout as pause } from "node:timers/promises";

const cache = process.env.GATEWAY_BUILD_CACHE;
if (!cache) throw new Error("Use the execution-owned GATEWAY_BUILD_CACHE");
const root = resolve(import.meta.dirname, "..");
const directory = join(cache, `admin-browser-${randomUUID()}`);
mkdirSync(directory, { mode: 0o700 });
const name = `gateway_test_browser_${randomUUID().replaceAll("-", "")}`;
const binary = join(
  process.env.CARGO_TARGET_DIR ?? join(cache, "cargo"),
  "debug/mcp-gateway-server",
);
const port = Number(process.env.GATEWAY_UI_TEST_PORT ?? 18081);
if (!Number.isInteger(port) || port < 1024 || port > 65535)
  throw new Error("Invalid isolated UI port");
const origin = `http://127.0.0.1:${port}`;
const PASSWORD = "owned-browser-fixture-password";
const key = join(directory, "vault-key.json");
function postgres(sql: string) {
  const r = spawnSync(
    "docker",
    [
      "compose",
      "-p",
      "cadre-gateway-tests",
      "-f",
      join(root, "deploy/compose.test.yaml"),
      "exec",
      "-T",
      "postgres",
      "psql",
      "-U",
      "gateway_test",
      "-d",
      "gateway_test",
      "-v",
      "ON_ERROR_STOP=1",
      "-c",
      sql,
    ],
    { stdio: "pipe" },
  );
  if (r.status !== 0)
    throw new Error("Owned PostgreSQL fixture command failed");
}
function command(args: string[], input?: string) {
  const r = spawnSync(binary, args, { input, stdio: ["pipe", "pipe", "pipe"] });
  if (r.status !== 0) throw new Error("Owned control-plane command failed");
}
const common = [
  "--database-url",
  `postgres://gateway_test:isolated-fixture-password@127.0.0.1:18432/${name}`,
  "--public-url",
  origin,
  "--vault-key-file",
  key,
  "--allow-loopback-http",
];
let server: ChildProcess | null = null;
let databaseCreated = false;
let cleaned = false;
async function cleanup() {
  if (cleaned) return;
  cleaned = true;
  const owned = server;
  if (owned) {
    owned.kill("SIGINT");
    await Promise.race([
      new Promise<void>((done) => {
        owned.once("exit", () => done());
        if (owned.exitCode !== null || owned.signalCode !== null) done();
      }),
      pause(5000),
    ]);
    if (owned.exitCode === null && owned.signalCode === null)
      owned.kill("SIGKILL");
  }
  if (databaseCreated) postgres(`DROP DATABASE ${name} WITH (FORCE)`);
  rmSync(directory, { recursive: true });
}
process.on("SIGINT", () => {
  void cleanup().then(() => process.exit(0));
});
process.on("SIGTERM", () => {
  void cleanup().then(() => process.exit(0));
});
async function request(
  path: string,
  token: string | null,
  method = "GET",
  body?: unknown,
) {
  const response = await fetch(origin + path, {
    method,
    redirect: "error",
    headers: {
      "content-type": "application/json",
      ...(token ? { authorization: `Bearer ${token}` } : {}),
    },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  if (!response.ok)
    throw new Error(`Fixture API failed: ${method} ${path} ${response.status}`);
  return response.status === 204
    ? null
    : ((await response.json()) as Record<string, unknown>);
}
try {
  postgres(`CREATE DATABASE ${name}`);
  databaseCreated = true;
  command(["generate-vault-key", "--output", key]);
  command(
    [
      ...common,
      "bootstrap-admin",
      "--email",
      "admin@example.test",
      "--name",
      "Administrator",
      "--password-stdin",
    ],
    PASSWORD,
  );
  const log = openSync(join(directory, "server.log"), "w", 0o600);
  server = spawn(binary, [...common, "--bind", `127.0.0.1:${port}`], {
    stdio: ["ignore", log, log],
  });
  closeSync(log);
  server.once("error", () => {
    void cleanup().then(() => process.exit(1));
  });
  let healthy = false;
  for (let i = 0; i < 50; i++) {
    try {
      if ((await fetch(origin + "/healthz")).ok) {
        healthy = true;
        break;
      }
    } catch {
      /* startup */
    }
    await pause(200);
  }
  if (!healthy) throw new Error("Owned control plane did not become healthy");
  const auth = await request("/api/endpoint/login", null, "POST", {
    email: "admin@example.test",
    password: PASSWORD,
  });
  const token = String(auth?.token);
  const revision = async () =>
    Number((await request("/api/admin/configuration", token))?.revision);
  const alice = await request("/api/admin/users", token, "POST", {
    expectedRevision: await revision(),
    email: "alice@example.test",
    name: "Alice",
    password: PASSWORD,
    role: "user",
  });
  const bob = await request("/api/admin/users", token, "POST", {
    expectedRevision: await revision(),
    email: "bob@example.test",
    name: "Bob",
    password: PASSWORD,
    role: "user",
  });
  const engineering = await request("/api/admin/groups", token, "POST", {
    expectedRevision: await revision(),
    name: "Engineering",
    memberIds: [alice?.id, bob?.id],
  });
  await request("/api/admin/groups", token, "POST", {
    expectedRevision: await revision(),
    name: "Documentation",
    memberIds: [alice?.id],
  });
  const local = "00000000-0000-4000-8000-000000000001";
  const remote = "00000000-0000-4000-8000-000000000002";
  const oauth = "00000000-0000-4000-8000-000000000003";
  for (const definition of [
    {
      id: local,
      name: "Local tools",
      description: "Owned browser fixture",
      active: true,
      transport: {
        kind: "stdio",
        command: { program: "/usr/bin/true", args: [], env: {}, cwd: null },
      },
      install: null,
      auth: { kind: "none" },
      toolPolicy: { mode: "allowlist", tools: ["echo"] },
    },
    {
      id: remote,
      name: "Remote tools",
      description: "Owned bearer definition fixture",
      active: true,
      transport: { kind: "http", url: "https://fixture.example/mcp" },
      install: null,
      auth: { kind: "bearer" },
      toolPolicy: { mode: "allowlist", tools: [] },
    },
    {
      id: oauth,
      name: "OAuth tools",
      description: "Owned OAuth definition fixture",
      active: true,
      transport: { kind: "http", url: "https://fixture.example/oauth-mcp" },
      install: null,
      auth: {
        kind: "oauth",
        clientId: "fixture-public-client",
        scopes: ["tools"],
      },
      toolPolicy: { mode: "denylist", tools: [] },
    },
  ])
    await request("/api/admin/mcps", token, "POST", {
      expectedRevision: await revision(),
      definition,
    });
  await request("/api/admin/assignments", token, "POST", {
    expectedRevision: await revision(),
    mcpId: local,
    target: { kind: "group", id: engineering?.id },
  });
  await request("/api/admin/assignments", token, "POST", {
    expectedRevision: await revision(),
    mcpId: oauth,
    target: { kind: "all" },
  });
  const aliceAuth = await request("/api/endpoint/login", null, "POST", {
    email: "alice@example.test",
    password: PASSWORD,
  });
  const aliceToken = String(aliceAuth?.token);
  await request(`/api/settings/${local}`, aliceToken, "PUT", {
    expectedRevision: await revision(),
    enabled: true,
    secretId: null,
  });
  await request("/api/endpoint/inventory", aliceToken, "POST", {
    mcpId: local,
    names: ["echo", "fail"],
  });
  // Owner metadata supports exact cleanup after interruption. Credentials are never written here.
  writeFileSync(
    join(directory, "owner.json"),
    JSON.stringify({
      fixturePid: process.pid,
      serverPid: server.pid,
      database: name,
      origin,
    }),
    { mode: 0o600 },
  );
  console.log(`Owned admin browser fixture ready: ${origin}`);
  console.log(`Cleanup owner: ${join(directory, "owner.json")}`);
} catch {
  await cleanup();
  throw new Error(
    "Admin browser fixture startup failed; owned resources cleaned",
  );
}
