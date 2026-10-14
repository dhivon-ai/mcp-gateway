import { describe, it, expect } from "vitest";
import * as decode from "./decode";
import { mcp as localMcp } from "@/local/contracts";

function definition(name: string) {
  return {
    id: "00000000-0000-4000-8000-000000000001",
    name,
    description: "",
    active: true,
    transport: {
      kind: "stdio",
      command: {
        program: "fixture",
        args: [],
        env: { ["K".repeat(129)]: "literal" },
        cwd: null,
      },
    },
    install: null,
    auth: { kind: "none" },
    toolPolicy: { mode: "allowlist", tools: [] },
  };
}
function local(name: string) {
  return {
    id: "00000000-0000-4000-8000-000000000001",
    name,
    description: "",
    transport: "stdio",
    auth: "none",
    enabled: false,
    secret: null,
    installationAvailable: false,
    installed: false,
    availability: "disabled",
    authRequired: false,
    toolPolicy: { mode: "allowlist", tools: [] },
  };
}
describe("Rust catalog wire bounds", () => {
  it.each(["a".repeat(200), "🙂".repeat(50)])(
    "accepts a server-valid 200-byte name in both websites",
    (name) => {
      expect(decode.mcp(definition(name)).name).toBe(name);
      expect(localMcp(local(name)).name).toBe(name);
    },
  );
  it.each(["a".repeat(201), "🙂".repeat(51)])(
    "rejects a name beyond the server byte limit",
    (name) => {
      expect(() => decode.mcp(definition(name))).toThrow();
      expect(() => localMcp(local(name))).toThrow();
    },
  );
  it("accepts long environment keys within the catalog budget and keeps object values typed", () => {
    const mcp = decode.mcp(definition("valid"));
    expect(mcp.transport).toMatchObject({
      command: { env: { ["K".repeat(129)]: "literal" } },
    });
    expect(() =>
      decode.command({
        program: "fixture",
        args: [],
        env: { ["K".repeat(decode.CATALOG_BYTE_LIMIT + 1)]: "x" },
        cwd: null,
      }),
    ).toThrow();
    expect(() =>
      decode.command({
        program: "fixture",
        args: [],
        env: { BAD: 3 },
        cwd: null,
      }),
    ).toThrow();
    expect(() =>
      decode.command({ program: "fixture", args: [3], env: {}, cwd: null }),
    ).toThrow();
  });
});
