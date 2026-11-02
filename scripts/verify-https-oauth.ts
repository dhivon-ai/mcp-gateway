/** Isolated Docker TLS → actual Mac → browser loopback OAuth rehearsal. */
import assert from "node:assert/strict";
import { spawn, spawnSync, type ChildProcess } from "node:child_process";
import { createHash, randomBytes, randomUUID } from "node:crypto";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  openSync,
  readFileSync,
  realpathSync,
  writeFileSync,
} from "node:fs";
import { request as httpsRequest } from "node:https";
import { createServer } from "node:net";
import { join, resolve } from "node:path";
import { createInterface } from "node:readline";
import { setTimeout as pause } from "node:timers/promises";

const cache = process.env.DHIVON_MCP_GATEWAY_BUILD_CACHE;
const archive = process.env.DHIVON_MCP_GATEWAY_TEST_CLI;
const fixture = process.env.DHIVON_MCP_GATEWAY_FIXTURE_BIN;
const tls = process.env.DHIVON_MCP_GATEWAY_TEST_TLS_DIR;
assert(
  cache && archive && fixture && tls,
  "Supply owned cache, extracted CLI, fixture and approved test TLS directory",
);
assert.equal(
  process.platform,
  "darwin",
  "This rehearsal verifies the real Mac client",
);
const root = resolve(import.meta.dirname, "..");
const directory = mkdtempSync(join(realpathSync(cache), "https-oauth-"));
const project = `gw-https-${randomUUID().replaceAll("-", "")}`;
const cli = join(directory, "dhivon-mcp-gateway");
copyFileSync(archive, cli); // retain the creating binary until verified Keychain cleanup
const native = join(directory, "native");
mkdirSync(native, { mode: 0o700 });
const state = realpathSync(native);
const common = ["--allow-fixture-http", "--fixture-state-path", state];
const image = "dhivon-mcp-gateway-server:0.1.0";
const proxyImage =
  "nginx:1.30.6-alpine@sha256:0e52bd054126cdc040b1276c0af347aeb02a41b9cfe305d6094767ac30e97949";
const log = openSync(join(directory, "private.log"), "wx", 0o600);
const password = randomBytes(32).toString("hex");
const ca = readFileSync(join(tls, "ca.pem"));
const children: ChildProcess[] = [];
function run(args: string[], input?: string) {
  const r = spawnSync("docker", args, {
    input,
    timeout: 180_000,
    maxBuffer: 8 * 1024 * 1024,
  });
  if (r.stderr) writeFileSync(log, r.stderr);
  assert.equal(
    r.status,
    0,
    "Owned Docker operation failed; inspect private.log",
  );
  return r.stdout;
}
function child(binary: string, args: string[]) {
  const p = spawn(binary, args, { stdio: ["pipe", "pipe", log] });
  children.push(p);
  p.on("error", () => {}); // readiness/exit checks report fixed errors
  return p;
}
async function stop(p: ChildProcess) {
  if (p.exitCode !== null || p.signalCode !== null) return;
  p.kill("SIGINT");
  await Promise.race([
    new Promise<void>((done) => p.once("exit", () => done())),
    pause(5000),
  ]);
  assert(
    p.exitCode !== null || p.signalCode !== null,
    "Owned child remains; retain recovery directory",
  );
}
async function port() {
  const s = createServer();
  await new Promise<void>((done, fail) => {
    s.once("error", fail);
    s.listen(0, "127.0.0.1", done);
  });
  const a = s.address();
  assert(a && typeof a === "object");
  await new Promise<void>((done, fail) =>
    s.close((e) => (e ? fail(e) : done())),
  );
  return a.port;
}
function object(v: unknown): Record<string, unknown> {
  assert(v && typeof v === "object" && !Array.isArray(v));
  return v as Record<string, unknown>;
}
async function wire(
  url: string,
  method = "GET",
  token?: string,
  body?: unknown,
) {
  return new Promise<{ status: number; body: Buffer }>((done, fail) => {
    const r = httpsRequest(
      url,
      {
        method,
        ca,
        headers: {
          ...(token ? { authorization: `Bearer ${token}` } : {}),
          "content-type": "application/json",
        },
      },
      (response) => {
        const chunks: Buffer[] = [];
        let size = 0;
        response.on("data", (b: Buffer) => {
          size += b.length;
          if (size > 2 * 1024 * 1024)
            r.destroy(new Error("Fixture body bound"));
          else chunks.push(b);
        });
        response.on("error", fail);
        response.on("end", () =>
          done({
            status: response.statusCode ?? 0,
            body: Buffer.concat(chunks),
          }),
        );
      },
    );
    r.setTimeout(10_000, () => r.destroy(new Error("Fixture HTTPS deadline")));
    r.on("error", fail);
    r.end(body === undefined ? undefined : JSON.stringify(body));
  });
}
const centralPort = await port(),
  upstreamPort = await port(),
  fixturePort = await port(),
  uiPort = await port(),
  privatePort = await port();
const central = `https://localhost:${centralPort}`,
  upstream = `https://localhost:${upstreamPort}`;
const localOrigin = `http://127.0.0.1:${uiPort}`,
  callback = `${localOrigin}/oauth/callback`;
const key = join(directory, "vault-keys.json"),
  env = join(directory, "deployment.env"),
  override = join(directory, "tls.yaml");
const config = join(directory, "nginx.conf");
writeFileSync(
  env,
  `POSTGRES_PASSWORD=${password}\nDHIVON_MCP_GATEWAY_PUBLIC_URL=${central}\nDHIVON_MCP_GATEWAY_KEY_FILE=${key}\nDHIVON_MCP_GATEWAY_IMAGE=${image}\nDHIVON_MCP_GATEWAY_PORT=${privatePort}\nDHIVON_MCP_GATEWAY_UID=${process.getuid?.()}\nDHIVON_MCP_GATEWAY_GID=${process.getgid?.()}\n`,
  { mode: 0o600 },
);
writeFileSync(
  config,
  `events {}\nhttp { access_log off; server { listen 8443 ssl; ssl_certificate /tls/server.pem; ssl_certificate_key /tls/server.key; location / { proxy_pass http://dhivon:8080; proxy_set_header Host $http_host; } } server { listen 9443 ssl; ssl_certificate /tls/server.pem; ssl_certificate_key /tls/server.key; location / { proxy_pass http://host.docker.internal:${fixturePort}; proxy_set_header Host 127.0.0.1:${fixturePort}; proxy_buffering off; proxy_read_timeout 60s; } } }`,
);
writeFileSync(
  override,
  JSON.stringify({
    services: {
      tls: {
        image: proxyImage,
        depends_on: { dhivon: { condition: "service_healthy" } },
        ports: [
          `127.0.0.1:${centralPort}:8443`,
          `127.0.0.1:${upstreamPort}:9443`,
        ],
        volumes: [
          `${config}:/etc/nginx/nginx.conf:ro`,
          `${resolve(tls)}:/tls:ro`,
        ],
      },
    },
  }),
); // JSON is a YAML subset
function compose(args: string[], input?: string) {
  return run(
    [
      "compose",
      "--env-file",
      env,
      "-p",
      project,
      "-f",
      join(root, "deploy/compose.yaml"),
      "-f",
      override,
      ...args,
    ],
    input,
  );
}
let cookie = "",
  csrf = "",
  generation = "",
  ui: ChildProcess | undefined,
  succeeded = false;
async function local(path: string, method = "GET", body?: unknown) {
  const r = await fetch(localOrigin + path, {
    method,
    signal: AbortSignal.timeout(30_000),
    headers: {
      cookie,
      origin: localOrigin,
      "x-csrf-token": csrf,
      "content-type": "application/json",
      ...(generation ? { "x-endpoint-generation": generation } : {}),
    },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  assert.equal(r.status, 200, `Native ${method} ${path}`);
  return object(await r.json());
}
async function startUi() {
  ui = child(cli, [
    "ui",
    "--server",
    central,
    ...common,
    "--port",
    String(uiPort),
    "--no-open",
  ]);
  cookie = "";
  for (let i = 0; i < 100; i++) {
    try {
      const r = await fetch(localOrigin, { signal: AbortSignal.timeout(1000) });
      if (r.ok) {
        cookie = r.headers.get("set-cookie")?.split(";")[0] ?? "";
        break;
      }
    } catch {
      /* startup */
    }
    await pause(100);
  }
  assert(cookie, "Actual packaged local UI is ready");
  csrf = String((await local("/api/context")).csrfToken);
}
const mcp = randomUUID();
try {
  run([
    "run",
    "--rm",
    "--user",
    `${process.getuid?.()}:${process.getgid?.()}`,
    "--mount",
    `type=bind,source=${directory},target=/keys`,
    image,
    "generate-vault-key",
    "--output",
    "/keys/vault-keys.json",
  ]);
  compose(["up", "-d", "--wait", "postgres"]);
  compose(
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
  child(fixture, [
    "--transport",
    "http",
    "--auth",
    "oauth",
    "--bind",
    `127.0.0.1:${fixturePort}`,
    "--public-url",
    upstream,
  ]);
  compose(["up", "-d", "--wait", "dhivon", "tls"]);
  assert.equal((await wire(central + "/healthz")).status, 200);
  assert.equal((await wire(upstream + "/healthz")).status, 200);
  async function api(
    path: string,
    method = "GET",
    body?: unknown,
    token?: string,
  ) {
    const r = await wire(central + path, method, token, body);
    assert.equal(r.status, 200, `Central ${method} ${path}`);
    return object(JSON.parse(r.body.toString()) as unknown);
  }
  const admin = String(
    (
      await api("/api/endpoint/login", "POST", {
        email: "admin@example.test",
        password,
      })
    ).token,
  );
  const revision = async () =>
    Number(
      (await api("/api/admin/configuration", "GET", undefined, admin)).revision,
    );
  await api(
    "/api/admin/mcps",
    "POST",
    {
      expectedRevision: await revision(),
      definition: {
        id: mcp,
        name: "HTTPS controlled OAuth",
        description: "Isolated Docker HTTPS simulation",
        active: true,
        transport: { kind: "http", url: upstream + "/mcp" },
        install: null,
        auth: {
          kind: "oauth",
          clientId: "dhivon-mcp-gateway-fixture-client",
          scopes: ["tools"],
        },
        toolPolicy: { mode: "allowlist", tools: ["echo"] },
      },
    },
    admin,
  );
  await api(
    "/api/admin/assignments",
    "POST",
    { expectedRevision: await revision(), mcpId: mcp, target: { kind: "all" } },
    admin,
  );
  await startUi();
  const logged = await local("/api/auth/login", "POST", {
    email: "admin@example.test",
    password,
  });
  generation = String(object(logged.identity).generation);
  const action = async (path: string, input: unknown, method = "POST") =>
    local(path, method, { expectedGeneration: generation, input });
  const consent = await action(`/api/oauth/${mcp}/connect`, {
    expectedRevision: Number((await local("/api/catalog")).revision),
  });
  const authorization = new URL(String(consent.authorizationUrl));
  assert.equal(authorization.origin, upstream);
  assert.equal(authorization.searchParams.get("redirect_uri"), callback);
  assert.equal(authorization.searchParams.get("code_challenge_method"), "S256");
  assert.equal(authorization.searchParams.get("resource"), upstream + "/mcp");
  assert.equal(
    authorization.searchParams.get("client_id"),
    "dhivon-mcp-gateway-fixture-client",
  );
  for (const query of [
    new URLSearchParams({ code: "unused", state: "forged", iss: upstream }),
    new URLSearchParams({
      code: "unused",
      state: authorization.searchParams.get("state") ?? "",
      iss: "https://unrelated.invalid",
    }),
  ]) {
    const r = await fetch(`${callback}?${query}`, { redirect: "manual" });
    assert.equal(
      r.status,
      400,
      "Forged state/issuer must not consume the live flow",
    );
  }
  writeFileSync(
    join(directory, "browser.json"),
    JSON.stringify({
      authorizationUrl: authorization.href,
      callback,
      central,
      upstream,
    }),
    { mode: 0o600 },
  );
  console.log(
    `Open the authorizationUrl in ${join(directory, "browser.json")} in this Mac's browser.`,
  );
  let connected = false;
  for (let i = 0; i < 180; i++) {
    if ((await local(`/api/oauth/${mcp}/status`)).connected === true) {
      connected = true;
      break;
    }
    await pause(1000);
  }
  assert(
    connected,
    "Browser OAuth callback not completed before rehearsal deadline",
  );
  const before = await local(`/api/oauth/${mcp}/status`);
  const catalog = await local("/api/catalog");
  assert(Array.isArray(catalog.mcps));
  assert.equal(
    object(catalog.mcps.find((v) => object(v).id === mcp)).enabled,
    false,
    "Consent must not enable",
  );
  assert(ui);
  await stop(ui);
  await startUi();
  assert.equal(
    (await local(`/api/oauth/${mcp}/status`)).connected,
    true,
    "Credential survives management restart",
  );
  await action(
    `/api/settings/${mcp}`,
    {
      expectedRevision: Number((await local("/api/catalog")).revision),
      enabled: true,
      secretId: null,
    },
    "PUT",
  );
  const gateway = child(cli, ["mcp", ...common]);
  assert(gateway.stdin && gateway.stdout);
  const lines = createInterface({ input: gateway.stdout });
  const pending = new Map<number, (reply: Record<string, unknown>) => void>();
  let id = 0;
  lines.on("line", (line) => {
    const r = object(JSON.parse(line) as unknown);
    if (typeof r.id === "number") pending.get(r.id)?.(r);
  });
  async function rpc(method: string, params: unknown) {
    const requestId = ++id;
    return new Promise<Record<string, unknown>>((done, fail) => {
      const timer = setTimeout(() => {
        pending.delete(requestId);
        fail(new Error("Agent deadline"));
      }, 30_000);
      pending.set(requestId, (reply) => {
        clearTimeout(timer);
        pending.delete(requestId);
        done(reply);
      });
      gateway.stdin!.write(
        JSON.stringify({ jsonrpc: "2.0", id: requestId, method, params }) +
          "\n",
      );
    });
  }
  const initialized = object(
    (
      await rpc("initialize", {
        protocolVersion: "2025-11-25",
        capabilities: {},
        clientInfo: { name: "dhivon-https-rehearsal", version: "1" },
      })
    ).result,
  );
  assert.equal(initialized.protocolVersion, "2025-11-25");
  gateway.stdin.write(
    JSON.stringify({ jsonrpc: "2.0", method: "notifications/initialized" }) +
      "\n",
  );
  const name = `gw_${createHash("sha256").update(mcp).digest("hex").slice(0, 32)}_${createHash("sha256").update("echo").digest("hex")}`;
  const listed = object((await rpc("tools/list", {})).result);
  assert(Array.isArray(listed.tools));
  assert(listed.tools.some((t) => object(t).name === name));
  async function call() {
    const r = object(
      (await rpc("tools/call", { name, arguments: { message: "https-oauth" } }))
        .result,
    );
    assert.equal(object(r.structuredContent).message, "https-oauth");
  }
  await call();
  console.log(
    "PASS: HTTPS control-plane sync, browser loopback callback, PKCE/state/issuer, persisted credential and explicit enable/tool call.",
  );
  await pause(32_000);
  // Expiry fails closed. Wait for the existing 60s sync/reconciliation cycle;
  // discovery is safe to poll, whereas tool calls must never be replayed.
  let restored = false;
  for (let i = 0; i < 45; i++) {
    const current = object((await rpc("tools/list", {})).result);
    assert(Array.isArray(current.tools));
    if (current.tools.some((tool) => object(tool).name === name)) {
      restored = true;
      break;
    }
    await pause(2000);
  }
  assert(restored, "OAuth tool did not return after scheduled refresh");
  await call();
  const refreshed = await local(`/api/oauth/${mcp}/status`);
  assert(
    Number(refreshed.version) > Number(before.version),
    "Expired access token must refresh through the central CAS",
  );
  await action(`/api/oauth/${mcp}/disconnect`, {
    expectedRevision: Number((await local("/api/catalog")).revision),
    expectedVersion: Number(refreshed.version),
  });
  assert.equal((await local(`/api/oauth/${mcp}/status`)).connected, false);
  const denied = await rpc("tools/call", {
    name,
    arguments: { message: "denied" },
  });
  assert(denied.error);
  await stop(gateway);
  lines.close();
  console.log(
    "PASS: token refresh and disconnect revoke subsequent agent use.",
  );
  succeeded = true;
} finally {
  // Capture startup failures before removing this generated stack.
  const diagnostics = compose(["logs", "--no-color", "tls", "dhivon"]);
  writeFileSync(log, diagnostics);
  for (const p of children.reverse()) await stop(p);
  if (existsSync(join(state, "identity.json"))) {
    const logout = spawnSync(cli, ["logout", ...common], {
      timeout: 30_000,
      stdio: ["ignore", log, log],
    });
    assert.equal(
      logout.status,
      0,
      "Creating CLI logout; retain directory on failure",
    );
    const metadata = object(
      JSON.parse(readFileSync(join(state, "identity.json"), "utf8")) as unknown,
    );
    assert.equal(metadata.active, null);
    assert.deepEqual(metadata.credentialAccounts, []);
    assert.deepEqual(metadata.pendingCleanup, []);
  }
  compose(["down", "--volumes", "--remove-orphans"]);
  const captured = readFileSync(join(directory, "private.log"), "utf8");
  assert(
    !captured.includes(password),
    "Private fixture password appeared in logs",
  );
  writeFileSync(
    join(directory, "result.json"),
    JSON.stringify(
      {
        succeeded,
        central,
        upstream,
        callback,
        project,
        image,
        proxyImage,
        nativeCredentialsCleared: true,
        stacksRemoved: true,
        provider: "controlled fixture, not an external provider",
      },
      null,
      2,
    ),
  );
  console.log(
    `Rehearsal evidence retained at ${directory}; remove temporary certificate trust separately.`,
  );
}
