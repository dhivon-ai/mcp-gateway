import "@testing-library/jest-dom/vitest";
import { beforeEach, afterEach, describe, it, expect, vi } from "vitest";
import {
  render,
  screen,
  fireEvent,
  waitFor,
  cleanup,
  within,
} from "@testing-library/react";
import { AdminApp } from "./app";
import type { AdminConfiguration } from "@/generated/AdminConfiguration";
import type { UiContext } from "@/generated/UiContext";

const alice = {
  id: "alice",
  username: "alice",
  role: "user" as const,
  active: true,
};
const bob = { id: "bob", username: "bob", role: "user" as const, active: true };
const context: UiContext = {
  mode: "admin",
  user: { id: "admin", username: "admin", role: "admin", active: true },
  serverUrl: "https://central.example",
  online: true,
  expiresAt: 12345,
  csrfToken: "memory-only-csrf",
};
const config: AdminConfiguration = {
  revision: 7,
  mcps: [
    {
      id: "mcp",
      name: "Local tools",
      description: "Fixture",
      active: true,
      transport: {
        kind: "stdio",
        command: { program: "fixture", args: [], env: {}, cwd: null },
      },
      install: null,
      auth: { kind: "none" },
      toolPolicy: { mode: "allowlist", tools: ["echo"] },
    },
  ],
  assignments: [],
  inventory: [{ mcpId: "mcp", names: ["echo", "fail"], observedAt: 123 }],
};
function response(value: unknown, status = 200) {
  return new Response(value === null ? null : JSON.stringify(value), {
    status,
    headers: { "content-type": "application/json" },
  });
}
function api(
  initial = context,
  mutation?: (path: string, options: RequestInit) => Response,
) {
  const calls: { path: string; options: RequestInit }[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (path: string, options: RequestInit) => {
      calls.push({ path, options });
      if (options.method !== "GET" && options.method !== undefined) {
        return mutation?.(path, options) ?? response(null, 204);
      }
      switch (path) {
        case "/api/context":
          return response(initial);
        case "/api/admin/configuration":
          return response(config);
        case "/api/admin/users":
          return response([context.user, alice, bob]);
        case "/api/admin/groups":
          return response([
            {
              id: "engineering",
              name: "Engineering",
              memberIds: ["alice", "bob"],
            },
          ]);
        default:
          return response(
            { code: "internal", message: "Unexpected fixture path" },
            500,
          );
      }
    }),
  );
  return calls;
}
beforeEach(() => {
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe() {}
      unobserve() {}
      disconnect() {}
    },
  );
});
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  window.location.hash = "";
});
describe("Administrator journeys", () => {
  it("keeps ordinary users outside administrator reads and actions", async () => {
    const calls = api({ ...context, user: alice });
    render(<AdminApp />);
    expect(
      await screen.findByRole("heading", {
        name: "Administrator access required",
      }),
    ).toBeVisible();
    expect(calls.every((c) => !c.path.startsWith("/api/admin"))).toBe(true);
  });
  it("submits login with in-memory CSRF, clears the password after rejection and exposes no token", async () => {
    const calls = api({ ...context, user: null }, () =>
      response({ code: "unauthorized", message: "Login is required." }, 401),
    );
    render(<AdminApp />);
    fireEvent.change(await screen.findByLabelText("Username"), {
      target: { value: "alice" },
    });
    fireEvent.change(screen.getByLabelText("Password"), {
      target: { value: "fixture-password" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Sign in" }));
    expect(await screen.findByText("Login is required.")).toBeVisible();
    expect(screen.getByLabelText("Password")).toHaveValue("");
    const sent = calls.find((c) => c.path === "/api/auth/login");
    expect(sent?.options.headers).toMatchObject({
      "X-CSRF-Token": "memory-only-csrf",
    });
    expect(sent?.options.credentials).toBe("same-origin");
    expect(localStorage.length).toBe(0);
    expect(sessionStorage.length).toBe(0);
  });
  it("filters the catalog and sends selected original tool names with a frozen revision; conflict never retries", async () => {
    const calls = api(context, () =>
      response(
        { code: "conflict", message: "Current state changed.", revision: 8 },
        409,
      ),
    );
    render(<AdminApp />);
    await screen.findByRole("heading", { name: "MCP catalog" });
    fireEvent.change(screen.getByLabelText("Search MCPs"), {
      target: { value: "missing" },
    });
    expect(screen.getByText("No results")).toBeVisible();
    fireEvent.change(screen.getByLabelText("Search MCPs"), {
      target: { value: "" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Manage Local tools" }));
    fireEvent.keyDown(screen.getByRole("tab", { name: "Tools" }), {
      key: "Enter",
    });
    fireEvent.click(screen.getByRole("checkbox", { name: "echo" }));
    fireEvent.click(screen.getByRole("checkbox", { name: "fail" }));
    fireEvent.change(screen.getByLabelText("Add tool name"), {
      target: { value: "manual_tool" },
    });
    fireEvent.click(screen.getByRole("button", { name: /^Add$/ }));
    fireEvent.click(screen.getByRole("button", { name: "Save policy" }));
    await screen.findAllByText("Changes need review");
    const writes = calls.filter((c) => c.path === "/api/admin/mcps/mcp");
    expect(writes).toHaveLength(1);
    const body = JSON.parse(String(writes[0].options.body));
    expect(body.expectedRevision).toBe(7);
    expect(body.definition.toolPolicy).toEqual({
      mode: "allowlist",
      tools: ["fail", "manual_tool"],
    });
    expect(screen.getByRole("dialog")).toBeVisible();
    expect(
      within(screen.getByRole("dialog")).getByRole("button", {
        name: "Reload current state",
      }),
    ).toBeVisible();
  });
  it("changes group membership using stable user IDs and a revision-checked write", async () => {
    const calls = api(context, () =>
      response({
        id: "engineering",
        name: "Engineering",
        memberIds: ["alice"],
      }),
    );
    render(<AdminApp />);
    await screen.findByRole("heading", { name: "MCP catalog" });
    fireEvent.click(screen.getByRole("link", { name: "Groups" }));
    await screen.findByRole("heading", { name: "Groups" });
    fireEvent.click(
      screen.getByRole("button", { name: "Edit group Engineering" }),
    );
    fireEvent.click(screen.getByRole("checkbox", { name: "bob" }));
    fireEvent.click(screen.getByRole("button", { name: "Save group" }));
    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
    );
    const write = calls.find((c) => c.path === "/api/admin/groups/engineering");
    expect(JSON.parse(String(write?.options.body))).toEqual({
      expectedRevision: 7,
      name: "Engineering",
      memberIds: ["alice"],
    });
  });
  it("requires a concrete confirmation before deleting a group", async () => {
    const calls = api();
    render(<AdminApp />);
    await screen.findByRole("heading", { name: "MCP catalog" });
    fireEvent.click(screen.getByRole("link", { name: "Groups" }));
    await screen.findByRole("heading", { name: "Groups" });
    fireEvent.click(
      screen.getByRole("button", { name: "Edit group Engineering" }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Delete group" }));
    expect(screen.getByRole("alertdialog")).toBeVisible();
    expect(calls.some((c) => c.options.method === "DELETE")).toBe(false);
    fireEvent.click(
      within(screen.getByRole("alertdialog")).getByRole("button", {
        name: "Cancel",
      }),
    );
    expect(calls.some((c) => c.options.method === "DELETE")).toBe(false);
  });
});
