import { afterEach, expect, it, vi } from "vitest";
import { readAdminData } from "./read-data";

afterEach(() => vi.unstubAllGlobals());

function fixture() {
  const users = Array.from({ length: 10001 }, (_, i) => ({
    id: `user-${i}`,
    username: `user${i}`,
    role: "user",
    active: true,
  }));
  const groups = users.map((u, i) => ({
    id: `group-${i}`,
    name: `Group${i}`,
    memberIds: [u.id],
  }));
  const config = {
    revision: 17,
    mcps: [],
    inventory: [],
    assignments: groups.map((g, i) => ({
      id: `grant-${i}`,
      mcpId: "mcp",
      target: { kind: "group", id: g.id },
    })),
  };
  return { users, groups, config };
}
function serve(data: ReturnType<typeof fixture>) {
  const calls: string[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (path: string) => {
      calls.push(path);
      const value = path.endsWith("/users")
        ? data.users
        : path.endsWith("/groups")
          ? data.groups
          : data.config;
      return new Response(JSON.stringify(value), {
        headers: { "content-type": "application/json" },
      });
    }),
  );
  return calls;
}
it("loads complete large coherent admin read sets on initial and follow-up reads", async () => {
  const data = fixture();
  const calls = serve(data);
  for (let i = 0; i < 2; i++) {
    const result = await readAdminData();
    expect(result.users).toEqual(data.users);
    expect(result.groups).toEqual(data.groups);
    expect(result.config.assignments).toEqual(data.config.assignments);
    expect(result.config.revision).toBe(17);
  }
  expect(calls).toEqual(
    Array.from({ length: 2 }, () => [
      "/api/admin/configuration",
      "/api/admin/users",
      "/api/admin/groups",
      "/api/admin/configuration",
    ]).flat(),
  );
});
it.each(["users", "groups"] as const)(
  "rejects a malformed late %s row",
  async (kind) => {
    const data = fixture();
    if (kind === "users")
      data.users[10000] = { ...data.users[10000], role: "invalid" };
    else
      data.groups[10000] = {
        ...data.groups[10000],
        memberIds: Array(10001).fill("user"),
      };
    serve(data);
    await expect(readAdminData()).rejects.toMatchObject({
      code: "internal",
      message: "The server returned invalid data.",
    });
  },
);
