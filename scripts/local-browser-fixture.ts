/** Disposable real-Postgres/Keychain local UI fixture. No production DSN or account is accepted. */
import { spawn, spawnSync, type ChildProcess } from "node:child_process";
import { randomUUID } from "node:crypto";
import {
  mkdirSync,
  openSync,
  closeSync,
  rmSync,
  writeFileSync,
  readFileSync,
  realpathSync,
  copyFileSync,
} from "node:fs";
import { resolve, join } from "node:path";
import { setTimeout as pause } from "node:timers/promises";

const cache = process.env.GATEWAY_BUILD_CACHE;
if (!cache) throw new Error("Use the execution-owned GATEWAY_BUILD_CACHE");
const root = resolve(import.meta.dirname, "..");
const directory = join(cache, `local-browser-${randomUUID()}`);
mkdirSync(directory, { mode: 0o700 });
const name = `gateway_test_local_browser_${randomUUID().replaceAll("-", "")}`;
const binary = join(
  process.env.CARGO_TARGET_DIR ?? join(cache, "cargo"),
  "debug/mcp-gateway-server",
);
const port = Number(process.env.GATEWAY_UI_TEST_PORT ?? 18091);
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
const cli = join(directory, "mcp-gateway");
copyFileSync(
  join(
    process.env.CARGO_TARGET_DIR ?? join(cache, "cargo"),
    "debug/mcp-gateway",
  ),
  cli,
);
const fixture = join(
  process.env.CARGO_TARGET_DIR ?? join(cache, "cargo"),
  "debug/gateway-fixture",
);
const nativeRoot = join(directory, "native");
mkdirSync(nativeRoot, { mode: 0o700 });
const statePath = realpathSync(nativeRoot);
const children: ChildProcess[] = [];
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
  for (const child of children.reverse()) {
    if (child.exitCode === null && child.signalCode === null) {
      child.kill("SIGINT");
      await Promise.race([
        new Promise<void>((done) => child.once("exit", () => done())),
        pause(5000),
      ]);
    }
    if (child.exitCode === null && child.signalCode === null)
      throw new Error("Owned child did not exit; recovery journal retained");
  }
  const result = spawnSync(
    cli,
    ["logout", "--allow-fixture-http", "--fixture-state-path", statePath],
    { stdio: "pipe" },
  );
  if (result.status !== 0)
    throw new Error("Creating CLI cleanup failed; recovery journal retained");
  const metadata = JSON.parse(
    readFileSync(join(statePath, "identity.json"), "utf8"),
  );
  if (
    metadata.active !== null ||
    metadata.credentialAccounts.length ||
    metadata.pendingCleanup.length
  )
    throw new Error("Native cleanup incomplete; recovery journal retained");
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
  for (const [auth, fixturePort] of [
    ["none", 18093],
    ["bearer", 18094],
    ["oauth", 18095],
  ] as const) {
    const log = openSync(join(directory, `${auth}.log`), "w", 0o600);
    children.push(
      spawn(
        fixture,
        [
          "--transport",
          "http",
          "--auth",
          auth,
          "--bind",
          `127.0.0.1:${fixturePort}`,
        ],
        { stdio: ["ignore", log, log] },
      ),
    );
    closeSync(log);
    let healthy = false;
    for (let i = 0; i < 50; i++) {
      try {
        if ((await fetch(`http://127.0.0.1:${fixturePort}/healthz`)).ok) {
          healthy = true;
          break;
        }
      } catch {}
      await pause(100);
    }
    if (!healthy) throw new Error("Owned MCP fixture failed startup");
  }
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
        command: {
          program: fixture,
          args: ["--transport", "stdio"],
          env: {},
          cwd: null,
        },
      },
      install: { program: "/usr/bin/true", args: [], env: {}, cwd: null },
      auth: { kind: "none" },
      toolPolicy: { mode: "allowlist", tools: ["echo"] },
    },
    {
      id: remote,
      name: "Remote tools",
      description: "Owned bearer definition fixture",
      active: true,
      transport: { kind: "http", url: "http://127.0.0.1:18094/mcp" },
      install: null,
      auth: { kind: "bearer" },
      toolPolicy: { mode: "allowlist", tools: ["echo"] },
    },
    {
      id: oauth,
      name: "OAuth tools",
      description: "Owned OAuth definition fixture",
      active: true,
      transport: { kind: "http", url: "http://127.0.0.1:18095/mcp" },
      install: null,
      auth: {
        kind: "oauth",
        clientId: "gateway-fixture-client",
        scopes: ["tools"],
      },
      toolPolicy: { mode: "allowlist", tools: ["echo"] },
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
  await request("/api/admin/assignments", token, "POST", {
    expectedRevision: await revision(),
    mcpId: remote,
    target: { kind: "all" },
  });
  const localLog = openSync(join(directory, "local-ui.log"), "w", 0o600);
  const localUI = spawn(
    cli,
    [
      "ui",
      "--server",
      origin,
      "--allow-fixture-http",
      "--fixture-state-path",
      statePath,
      "--port",
      "18092",
      "--no-open",
    ],
    { stdio: ["ignore", localLog, localLog] },
  );
  children.push(localUI);
  closeSync(localLog);
  let localHealthy = false;
  for (let i = 0; i < 100; i++) {
    try {
      if ((await fetch("http://127.0.0.1:18092/")).ok) {
        localHealthy = true;
        break;
      }
    } catch {}
    await pause(200);
  }
  if (!localHealthy) throw new Error("Owned local UI failed startup");
  // Owner metadata supports exact cleanup after interruption. Credentials are never written here.
  writeFileSync(
    join(directory, "owner.json"),
    JSON.stringify({
      fixturePid: process.pid,
      serverPid: server.pid,
      database: name,
      origin,
      nativeRoot: statePath,
      children: children.map((child) => child.pid),
    }),
    { mode: 0o600 },
  );
  console.log(
    `Owned local browser fixture ready: http://127.0.0.1:18092/ (central ${origin})`,
  );
  console.log(`Cleanup owner: ${join(directory, "owner.json")}`);
} catch {
  console.error(`Owned fixture recovery directory: ${directory}`);
  await cleanup();
  throw new Error(
    "Local browser fixture startup failed; inspect retained owner journal if cleanup failed",
  );
}
