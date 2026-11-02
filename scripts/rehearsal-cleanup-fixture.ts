/** Controlled cleanup probe. Real Docker + extracted Mac CLI, no TLS trust changes. */
import assert from "node:assert/strict";
import { copyFileSync, readFileSync, writeFileSync } from "node:fs";
import { randomBytes } from "node:crypto";
import { createServer } from "node:net";
import { join } from "node:path";
import { Rehearsal } from "./rehearsal-lifecycle.ts";
const cache = process.env.DHIVON_MCP_GATEWAY_BUILD_CACHE;
const cli = process.env.DHIVON_MCP_GATEWAY_TEST_CLI;
const mode = process.argv[2];
assert(
  cache &&
    cli &&
    [
      "resources",
      "native",
      "normal",
      "diagnostics",
      "incomplete",
      "incomplete-native",
      "failure",
    ].includes(mode),
);
assert.equal(process.platform, "darwin");
const r = Rehearsal.create(cache, cli);
r.supervise();
process.send?.({ directory: r.directory });
async function port() {
  const server = createServer();
  await new Promise<void>((done) => server.listen(0, "127.0.0.1", done));
  const addr = server.address();
  assert(addr && typeof addr === "object");
  await new Promise<void>((done) => server.close(() => done()));
  return addr.port;
}
const central = `http://127.0.0.1:${await port()}`;
const local = `http://127.0.0.1:${await port()}`;
const password = randomBytes(32).toString("hex");
const image = "dhivon-mcp-gateway-server:0.1.0";
const env = `POSTGRES_PASSWORD=${password}\nDHIVON_MCP_GATEWAY_PUBLIC_URL=${central}\nDHIVON_MCP_GATEWAY_KEY_FILE=${join(r.directory, "vault-keys.json")}\nDHIVON_MCP_GATEWAY_IMAGE=${image}\nDHIVON_MCP_GATEWAY_PORT=${new URL(central).port}\nDHIVON_MCP_GATEWAY_UID=${process.getuid?.()}\nDHIVON_MCP_GATEWAY_GID=${process.getgid?.()}\n`;
writeFileSync(join(r.directory, "deployment.env"), env, { mode: 0o600 });
copyFileSync(
  join(import.meta.dirname, "../deploy/compose.yaml"),
  join(r.directory, "compose.yaml"),
);
writeFileSync(
  join(r.directory, "tls.yaml"),
  JSON.stringify({
    services: { dhivon: { command: ["--allow-loopback-http"] } },
  }),
  { mode: 0o600 },
);
r.seal(["deployment.env", "compose.yaml", "tls.yaml"]);
const command = r.command.bind(r);
r.command = (binary, args, options) => {
  if (mode === "incomplete-native" && args[0] === "logout")
    throw new Error("Controlled native cleanup fault");
  return command(binary, args, options);
};
const compose = r.compose.bind(r);
r.compose = (args, options) => {
  if (
    (mode === "diagnostics" && args[0] === "logs") ||
    (mode === "incomplete" && args[0] === "down")
  )
    throw new Error("Controlled cleanup fault");
  return compose(args, options);
};
try {
  await r.compose(["up", "-d", "--wait", "postgres"]);
  const stubborn = join(r.directory, "owned-idle.ts");
  writeFileSync(
    stubborn,
    'process.on("SIGINT", () => {}); setInterval(() => {}, 1000);',
  );
  r.child(process.execPath, [stubborn]);
  if (mode !== "resources") {
    await r.command("docker", [
      "run",
      "--rm",
      "--name",
      `${r.journal.project}-keygen`,
      "--label",
      `dhivon.rehearsal=${r.journal.project}`,
      "--user",
      `${process.getuid?.()}:${process.getgid?.()}`,
      "--mount",
      `type=bind,source=${r.directory},target=/keys`,
      image,
      "generate-vault-key",
      "--output",
      "/keys/vault-keys.json",
    ]);
    await r.compose(
      [
        "run",
        "--rm",
        "--no-deps",
        "-T",
        "dhivon",
        "--allow-loopback-http",
        "bootstrap-admin",
        "--email",
        "admin@example.test",
        "--name",
        "Administrator",
        "--password-stdin",
      ],
      { input: password },
    );
    await r.compose(["up", "-d", "--wait", "dhivon"]);
    r.child(r.journal.cli, [
      "ui",
      "--server",
      central,
      "--allow-fixture-http",
      "--fixture-state-path",
      r.journal.state,
      "--port",
      new URL(local).port,
      "--no-open",
    ]);
    let cookie = "";
    for (let i = 0; i < 100; i++) {
      try {
        const response = await fetch(local, {
          signal: AbortSignal.any([r.signal, AbortSignal.timeout(1000)]),
        });
        if (response.ok) {
          cookie = response.headers.get("set-cookie")?.split(";")[0] ?? "";
          break;
        }
      } catch {
        /* readiness */
      }
      await r.wait(100);
    }
    assert(cookie);
    const context = await fetch(`${local}/api/context`, {
      headers: { cookie },
      signal: r.signal,
    });
    const { csrfToken } = (await context.json()) as { csrfToken: string };
    const response = await fetch(`${local}/api/auth/login`, {
      method: "POST",
      headers: {
        cookie,
        origin: local,
        "x-csrf-token": csrfToken,
        "content-type": "application/json",
      },
      body: JSON.stringify({ email: "admin@example.test", password }),
      signal: r.signal,
    });
    assert.equal(response.status, 200, "Actual native login");
    const metadata = JSON.parse(
      readFileSync(join(r.journal.state, "identity.json"), "utf8"),
    ) as { active: unknown; credentialAccounts: unknown[] };
    assert(
      metadata.active && metadata.credentialAccounts.length > 0,
      "Actual Keychain ownership established",
    );
  }
  process.send?.({ ready: mode });
  if (["resources", "native"].includes(mode)) await r.wait(120_000);
  if (mode === "failure")
    throw new Error("Controlled ordinary workflow failure");
  r.journal.workflowSucceeded = true;
} catch {
  process.exitCode = 1;
} finally {
  r.save();
  const first = r.cleanup();
  assert.equal(
    first,
    r.cleanup(),
    "Concurrent/repeated teardown shares one promise",
  );
  if (!(await first) || !r.journal.cleanup.diagnostics?.ok)
    process.exitCode = 1;
  if (r.journal.interrupted)
    process.exitCode = r.journal.interrupted === "SIGINT" ? 130 : 143;
  process.disconnect?.();
}
