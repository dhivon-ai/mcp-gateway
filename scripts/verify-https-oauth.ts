/** Isolated Docker TLS → actual Mac → browser loopback OAuth rehearsal. */
import assert from "node:assert/strict";
import { type ChildProcess } from "node:child_process";
import { createHash, randomBytes, randomUUID } from "node:crypto";
import { copyFileSync, readFileSync, writeFileSync } from "node:fs";
import { request as httpsRequest } from "node:https";
import { createServer } from "node:net";
import { join, resolve } from "node:path";
import { createInterface } from "node:readline";
import { Rehearsal } from "./rehearsal-lifecycle.ts";

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
const rehearsal = Rehearsal.create(cache, archive);
rehearsal.supervise();
const { directory } = rehearsal;
console.log(`Ownership record: ${directory}/ownership.json`);
const { project, cli, state } = rehearsal.journal;
const common = ["--allow-fixture-http", "--fixture-state-path", state];
const child = (binary: string, args: string[]) => rehearsal.child(binary, args);
const stop = (p: ChildProcess) => rehearsal.stop(p);
const pause = (ms: number) => rehearsal.wait(ms);
const image = "dhivon-mcp-gateway-server:0.1.0";
const proxyImage =
  "nginx:1.30.6-alpine@sha256:0e52bd054126cdc040b1276c0af347aeb02a41b9cfe305d6094767ac30e97949";
const password = randomBytes(32).toString("hex");
const ca = readFileSync(join(tls, "ca.pem"));
const run = (args: string[], input?: string) =>
  rehearsal.command("docker", args, { input });
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
        signal: rehearsal.signal,
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
copyFileSync(
  join(root, "deploy/compose.yaml"),
  join(directory, "compose.yaml"),
);
rehearsal.journal.details = {
  central,
  upstream,
  callback,
  image,
  proxyImage,
  provider: "controlled fixture, not an external provider",
};
rehearsal.seal(["compose.yaml", "tls.yaml", "deployment.env", "nginx.conf"]);
const compose = (args: string[], input?: string) =>
  rehearsal.compose(args, { input });
let cookie = "",
  csrf = "",
  generation = "",
  ui: ChildProcess | undefined,
  succeeded = false;
async function local(path: string, method = "GET", body?: unknown) {
  const r = await fetch(localOrigin + path, {
    method,
    signal: AbortSignal.any([rehearsal.signal, AbortSignal.timeout(30_000)]),
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
      const r = await fetch(localOrigin, {
        signal: AbortSignal.any([rehearsal.signal, AbortSignal.timeout(1000)]),
      });
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
  await run([
    "run",
    "--name",
    `${project}-keygen`,
    "--label",
    `dhivon.rehearsal=${project}`,
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
  await compose(["up", "-d", "--wait", "postgres"]);
  await compose(
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
  await compose(["up", "-d", "--wait", "dhivon", "tls"]);
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
    const r = await fetch(`${callback}?${query}`, {
      redirect: "manual",
      signal: AbortSignal.any([rehearsal.signal, AbortSignal.timeout(10_000)]),
    });
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
  const pending = new Map<
    number,
    {
      done: (reply: Record<string, unknown>) => void;
    }
  >();
  let id = 0;
  lines.on("line", (line) => {
    try {
      const r = object(JSON.parse(line) as unknown);
      if (typeof r.id === "number") pending.get(r.id)?.done(r);
    } catch {
      rehearsal.abort.abort(new Error("Malformed agent response"));
    }
  });
  async function rpc(method: string, params: unknown) {
    const requestId = ++id;
    return new Promise<Record<string, unknown>>((done, fail) => {
      const timer = setTimeout(() => {
        settle();
        fail(new Error("Agent deadline"));
      }, 30_000);
      const settle = () => {
        clearTimeout(timer);
        pending.delete(requestId);
        rehearsal.signal.removeEventListener("abort", cancel);
      };
      const cancel = () => {
        settle();
        fail(new Error("Agent interrupted"));
      };
      rehearsal.signal.addEventListener("abort", cancel, { once: true });
      pending.set(requestId, {
        done: (reply) => {
          settle();
          done(reply);
        },
      });
      if (rehearsal.signal.aborted) {
        cancel();
        return;
      }
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
} catch {
  console.error(
    "Rehearsal failed or interrupted; inspect retained private evidence.",
  );
  process.exitCode = 1;
} finally {
  try {
    rehearsal.journal.workflowSucceeded = succeeded;
    rehearsal.save();
    const clean = await rehearsal.cleanup();
    const result = object(
      JSON.parse(
        readFileSync(join(directory, "result.json"), "utf8"),
      ) as unknown,
    );
    const redacted = !readFileSync(
      join(directory, "private.log"),
      "utf8",
    ).includes(password);
    if (!redacted) {
      rehearsal.journal.workflowSucceeded = false;
      rehearsal.save();
    }
    writeFileSync(
      join(directory, "result.json"),
      JSON.stringify(
        {
          ...result,
          succeeded: result.succeeded === true && redacted,
          central,
          upstream,
          callback,
          image,
          proxyImage,
          logsRedacted: redacted,
          provider: "controlled fixture, not an external provider",
        },
        null,
        2,
      ),
      { mode: 0o600 },
    );
    if (!clean || !redacted || !rehearsal.journal.cleanup.diagnostics?.ok)
      process.exitCode = 1;
    if (rehearsal.journal.interrupted)
      process.exitCode = rehearsal.journal.interrupted === "SIGINT" ? 130 : 143;
    console.log(
      `Rehearsal evidence retained at ${directory}; remove temporary certificate trust separately.`,
    );
  } finally {
    rehearsal.close();
  }
}
