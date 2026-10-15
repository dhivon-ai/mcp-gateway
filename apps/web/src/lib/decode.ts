import type { UserInfo } from "@/generated/UserInfo";
import type { GroupInfo } from "@/generated/GroupInfo";
import type { CommandSpec } from "@/generated/CommandSpec";
import type { McpDefinition } from "@/generated/McpDefinition";
import type { Assignment } from "@/generated/Assignment";
import type { AdminConfiguration } from "@/generated/AdminConfiguration";
import type { UiContext } from "@/generated/UiContext";
import type { RevisionInfo } from "@/generated/RevisionInfo";
import type { GrantTarget } from "@/generated/GrantTarget";
import type { AuthMode } from "@/generated/AuthMode";
import type { Transport } from "@/generated/Transport";

function invalid(): never {
  throw new Error("Invalid response contract");
}
export function object(value: unknown): Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value))
    return invalid();
  return value as Record<string, unknown>;
}
const utf8 = new TextEncoder();
// Rust wire limits count UTF-8 bytes. The whole serialized catalog is capped at 1 MiB.
export const CATALOG_BYTE_LIMIT = 1048576;
export const MCP_NAME_BYTE_LIMIT = 200;
export function string(value: unknown, max = 65536): string {
  return typeof value === "string" &&
    value.length <= max &&
    utf8.encode(value).length <= max
    ? value
    : invalid();
}
export function bool(value: unknown): boolean {
  return typeof value === "boolean" ? value : invalid();
}
export function integer(value: unknown, max = Number.MAX_SAFE_INTEGER): number {
  return typeof value === "number" &&
    Number.isSafeInteger(value) &&
    value >= 0 &&
    value <= max
    ? value
    : invalid();
}
export function nullable<T>(
  value: unknown,
  decode: (value: unknown) => T,
): T | null {
  return value === null ? null : decode(value);
}
export function array<T>(
  value: unknown,
  decode: (value: unknown) => T,
  // Admin users/groups/assignments have no server row limit. Bounded wire fields
  // supply their own limit; every element is still decoded in either case.
  max = Number.MAX_SAFE_INTEGER,
): T[] {
  return Array.isArray(value) && value.length <= max
    ? value.map(decode)
    : invalid();
}
export function user(value: unknown): UserInfo {
  const v = object(value);
  if (v.role !== "admin" && v.role !== "user") return invalid();
  return {
    id: string(v.id, 64),
    email: string(v.email, 254),
    name: string(v.name, 200),
    role: v.role,
    active: bool(v.active),
  };
}
export function group(value: unknown): GroupInfo {
  const v = object(value);
  return {
    id: string(v.id, 64),
    name: string(v.name, 128),
    memberIds: array(v.memberIds, (v) => string(v, 64), 10000),
  };
}
export function command(value: unknown): CommandSpec {
  const v = object(value);
  const env = object(v.env);
  const entries = Object.entries(env);
  if (entries.length > 128) return invalid();
  return {
    program: string(v.program, 4096),
    args: array(v.args, (v) => string(v, 16384), 128),
    env: Object.fromEntries(
      entries.map(([k, v]) => [
        string(k, CATALOG_BYTE_LIMIT),
        string(v, 16384),
      ]),
    ),
    cwd: nullable(v.cwd, (v) => string(v, 4096)),
  };
}
function transport(value: unknown): Transport {
  const v = object(value);
  if (v.kind === "stdio") return { kind: "stdio", command: command(v.command) };
  if (v.kind === "http") return { kind: "http", url: string(v.url, 4096) };
  return invalid();
}
function auth(value: unknown): AuthMode {
  const v = object(value);
  if (v.kind === "none" || v.kind === "bearer") return { kind: v.kind };
  if (v.kind === "oauth")
    return {
      kind: "oauth",
      clientId: string(v.clientId, 512),
      scopes: array(v.scopes, (v) => string(v, 512), 128),
    };
  return invalid();
}
export function mcp(value: unknown): McpDefinition {
  const v = object(value);
  const p = object(v.toolPolicy);
  if (p.mode !== "allowlist" && p.mode !== "denylist") return invalid();
  return {
    id: string(v.id, 64),
    name: string(v.name, MCP_NAME_BYTE_LIMIT),
    description: string(v.description, 4096),
    active: bool(v.active),
    transport: transport(v.transport),
    install: nullable(v.install, command),
    auth: auth(v.auth),
    toolPolicy: {
      mode: p.mode,
      tools: array(p.tools, (v) => string(v, 512), 2048),
    },
  };
}
export function target(value: unknown): GrantTarget {
  const v = object(value);
  if (v.kind === "all") return { kind: "all" };
  if (v.kind === "user" || v.kind === "group")
    return { kind: v.kind, id: string(v.id, 64) };
  return invalid();
}
export function assignment(value: unknown): Assignment {
  const v = object(value);
  return {
    id: string(v.id, 64),
    mcpId: string(v.mcpId, 64),
    target: target(v.target),
  };
}
export function configuration(value: unknown): AdminConfiguration {
  const v = object(value);
  return {
    revision: integer(v.revision, 4294967295),
    mcps: array(v.mcps, mcp, 512),
    assignments: array(v.assignments, assignment),
    inventory: array(
      v.inventory,
      (x) => {
        const i = object(x);
        return {
          mcpId: string(i.mcpId, 64),
          names: array(i.names, (v) => string(v, 512), 2048),
          observedAt: integer(i.observedAt),
        };
      },
      512,
    ),
  };
}
export function context(value: unknown): UiContext {
  const v = object(value);
  return {
    mode: string(v.mode, 32),
    user: nullable(v.user, user),
    serverUrl: nullable(v.serverUrl, (v) => string(v, 4096)),
    online: bool(v.online),
    expiresAt: nullable(v.expiresAt, integer),
    csrfToken: nullable(v.csrfToken, (v) => string(v, 128)),
  };
}
export function revision(value: unknown): RevisionInfo {
  return { revision: integer(object(value).revision, 4294967295) };
}
export function empty(value: unknown): null {
  return value === null ? null : invalid();
}
