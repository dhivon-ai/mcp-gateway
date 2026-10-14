import { object, string, bool, integer, nullable, array } from "@/lib/decode";
import type { LocalCatalog } from "@/generated/LocalCatalog";
import type { LocalMcpMetadata } from "@/generated/LocalMcpMetadata";
import type { SecretMetadata } from "@/generated/SecretMetadata";
import type { SecretMutationOutcome } from "@/generated/SecretMutationOutcome";
import type { InstallProgress } from "@/generated/InstallProgress";
import type { InstallState } from "@/generated/InstallState";
import type { LocalOAuthStatus } from "@/generated/LocalOAuthStatus";
import type { OAuthConsent } from "@/generated/OAuthConsent";

export type EndpointStatus = {
  identity: {
    state: string;
    generation: string | null;
    username: string | null;
    sessionExpiresAt: number | null;
  };
  configuration: {
    state: string;
    revision: number | null;
    validatedAt: number | null;
    expiresAt: number | null;
    remainingSeconds: number | null;
  };
  online: boolean;
  lastSyncError: string | null;
};
export type LocalContext = {
  mode: "local";
  status: EndpointStatus;
  csrfToken: string;
  serverUrl: string;
};
export function status(value: unknown): EndpointStatus {
  const v = object(value),
    i = object(v.identity),
    c = object(v.configuration);
  return {
    identity: {
      state: string(i.state, 64),
      generation: nullable(i.generation, (v) => string(v, 64)),
      username: nullable(i.username, (v) => string(v, 64)),
      sessionExpiresAt: nullable(i.sessionExpiresAt, integer),
    },
    configuration: {
      state: string(c.state, 64),
      revision: nullable(c.revision, (v) => integer(v, 4294967295)),
      validatedAt: nullable(c.validatedAt, integer),
      expiresAt: nullable(c.expiresAt, integer),
      remainingSeconds: nullable(c.remainingSeconds, (v) => integer(v, 3600)),
    },
    online: bool(v.online),
    lastSyncError: nullable(v.lastSyncError, (v) => string(v, 64)),
  };
}
export function context(value: unknown): LocalContext {
  const v = object(value);
  if (v.mode !== "local") throw new Error("Wrong interface");
  return {
    mode: "local",
    status: status(v.status),
    csrfToken: string(v.csrfToken, 128),
    serverUrl: string(v.serverUrl, 4096),
  };
}
export function mcp(value: unknown): LocalMcpMetadata {
  const v = object(value),
    p = object(v.toolPolicy);
  if (
    !["stdio", "http"].includes(string(v.transport)) ||
    !["none", "bearer", "oauth"].includes(string(v.auth)) ||
    (p.mode !== "allowlist" && p.mode !== "denylist")
  )
    throw new Error("Invalid MCP");
  return {
    id: string(v.id, 64),
    name: string(v.name, 128),
    description: string(v.description, 4096),
    transport: string(v.transport),
    auth: string(v.auth),
    enabled: bool(v.enabled),
    secret: nullable(v.secret, (x) => {
      const s = object(x);
      return {
        id: string(s.id, 64),
        key: string(s.key, 128),
        version: integer(s.version),
      };
    }),
    installationAvailable: bool(v.installationAvailable),
    installed: bool(v.installed),
    availability: string(v.availability, 64),
    authRequired: bool(v.authRequired),
    toolPolicy: {
      mode: p.mode,
      tools: array(p.tools, (v) => string(v, 512), 2048),
    },
  };
}
export function catalog(value: unknown): LocalCatalog {
  const v = object(value);
  return {
    generation: string(v.generation, 64),
    revision: integer(v.revision, 4294967295),
    expiresAt: integer(v.expiresAt),
    mcps: array(v.mcps, mcp, 512),
  };
}
export function secret(value: unknown): SecretMetadata {
  const v = object(value);
  return {
    id: string(v.id, 64),
    key: string(v.key, 128),
    version: integer(v.version),
    updatedAt: integer(v.updatedAt),
  };
}
export function secrets(value: unknown): SecretMetadata[] {
  return array(value, secret, 1024);
}
export function outcome(value: unknown): SecretMutationOutcome {
  const v = object(value);
  return {
    confirmed: bool(v.confirmed),
    synchronized: bool(v.synchronized),
    metadata: nullable(v.metadata, secret),
  };
}
const installStates: InstallState[] = [
  "running",
  "failed",
  "cancelled",
  "expired",
  "revoked",
  "installed_disabled",
  "enabled",
  "interrupted",
];
export function progress(value: unknown): InstallProgress {
  const v = object(value);
  const state = string(v.state);
  if (!installStates.includes(state as InstallState))
    throw new Error("Invalid progress");
  const exit = v.exitCode;
  if (
    exit !== null &&
    (typeof exit !== "number" || !Number.isSafeInteger(exit))
  )
    throw new Error("Invalid exit code");
  return {
    id: string(v.id, 64),
    mcpId: string(v.mcpId, 64),
    generation: string(v.generation, 64),
    state: state as InstallState,
    startedAt: integer(v.startedAt),
    completedAt: nullable(v.completedAt, integer),
    elapsedSeconds: integer(v.elapsedSeconds),
    exitCode: exit,
    message: string(v.message, 512),
  };
}
export function oauth(value: unknown): LocalOAuthStatus {
  const v = object(value);
  return {
    connected: bool(v.connected),
    version: nullable(v.version, integer),
    expiresAt: nullable(v.expiresAt, integer),
    readiness: string(v.readiness, 64),
  };
}
export function consent(value: unknown): OAuthConsent {
  const v = object(value);
  return {
    flowId: string(v.flowId, 64),
    authorizationUrl: string(v.authorizationUrl, 16384),
  };
}
export function confirmed(value: unknown): boolean {
  if (object(value).confirmed !== true) throw new Error("Not confirmed");
  return true;
}
export function acknowledgement(value: unknown): boolean {
  const v = object(value);
  if (v.cancelled !== true && v.cancellationRequested !== true)
    throw new Error("Not acknowledged");
  return true;
}
export function remaining(
  context: LocalContext,
  observed: number,
  now: number,
): number {
  const c = context.status.configuration;
  if (
    c.state !== "valid" ||
    c.expiresAt === null ||
    c.remainingSeconds === null
  )
    return 0;
  return Math.max(
    0,
    Math.min(
      c.remainingSeconds - Math.max(0, (now - observed) / 1000),
      c.expiresAt - now / 1000,
    ),
  );
}
export function countdown(seconds: number): string {
  const n = Math.ceil(Math.max(0, seconds));
  return `${Math.floor(n / 60)
    .toString()
    .padStart(2, "0")}:${(n % 60).toString().padStart(2, "0")}`;
}
export function segment(id: string): string {
  if (!/^[A-Za-z0-9_-]{1,64}$/.test(id)) throw new Error("Invalid identifier");
  return id;
}
