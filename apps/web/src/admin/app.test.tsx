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
  name: "alice",
  email: "alice@example.test",
  role: "user" as const,
  active: true,
};
const bob = {
  id: "bob",
  name: "bob",
  email: "bob@example.test",
  role: "user" as const,
  active: true,
};
const context: UiContext = {
  mode: "admin",
  user: {
    id: "admin",
    name: "admin",
    email: "admin@example.test",
    role: "admin",
    active: true,
  },
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
  configuration = config,
  read?: (path: string) => Response | undefined,
) {
  const calls: { path: string; options: RequestInit }[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (path: string, options: RequestInit) => {
      calls.push({ path, options });
      if (options.method !== "GET" && options.method !== undefined) {
        return mutation?.(path, options) ?? response(null, 204);
      }
      const override = read?.(path);
      if (override) return override;
      switch (path) {
        case "/api/context":
          return response(initial);
        case "/api/admin/configuration":
          return response(configuration);
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
  it.each(["create", "edit"])(
    "%s uses distinct display name and email profile fields",
    async (mode) => {
      const calls = api(context, () =>
        response({
          ...alice,
          name: "Display Name",
          email: "display@example.test",
        }),
      );
      render(<AdminApp />);
      await screen.findByRole("heading", { name: "MCP catalog" });
      fireEvent.click(screen.getByRole("link", { name: "Users" }));
      fireEvent.click(
        await screen.findByRole("button", {
          name:
            mode === "create"
              ? "Create user"
              : "Edit user alice (alice@example.test)",
        }),
      );
      const dialog = screen.getByRole("dialog");
      fireEvent.change(within(dialog).getByLabelText("Name"), {
        target: { value: "Display Name" },
      });
      const email = within(dialog).getByLabelText("Email");
      expect(email).toHaveAttribute("type", "email");
      fireEvent.change(email, { target: { value: "display@example.test" } });
      if (mode === "create")
        fireEvent.change(within(dialog).getByLabelText("Password"), {
          target: { value: "fixture-password" },
        });
      else
        expect(
          within(dialog).queryByLabelText("Password"),
        ).not.toBeInTheDocument();
      fireEvent.click(
        within(dialog).getByRole("button", {
          name: mode === "create" ? "Create user" : "Save user",
        }),
      );
      await waitFor(() =>
        expect(
          calls.filter(
            (c) => c.options.method === (mode === "create" ? "POST" : "PUT"),
          ),
        ).toHaveLength(1),
      );
      const write = calls.find(
        (c) => c.options.method === (mode === "create" ? "POST" : "PUT"),
      );
      expect(JSON.parse(String(write?.options.body))).toEqual({
        expectedRevision: 7,
        name: "Display Name",
        email: "display@example.test",
        role: "user",
        ...(mode === "create"
          ? { password: "fixture-password" }
          : { active: true }),
      });
    },
  );
  it("keeps existing passwords absent and resets through the protected write-only action", async () => {
    const calls = api(context, () => response(null, 204));
    render(<AdminApp />);
    await screen.findByRole("heading", { name: "MCP catalog" });
    fireEvent.click(screen.getByRole("link", { name: "Users" }));
    fireEvent.click(
      await screen.findByRole("button", {
        name: "Edit user alice (alice@example.test)",
      }),
    );
    fireEvent.click(screen.getByRole("button", { name: "Reset password" }));
    fireEvent.change(screen.getByLabelText("New password"), {
      target: { value: "new-fixture-password" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Reset password" }));
    await waitFor(() =>
      expect(calls.filter((c) => c.options.method === "PUT")).toHaveLength(1),
    );
    const write = calls.find((c) => c.options.method === "PUT");
    expect(write?.path).toBe("/api/admin/users/alice/password");
    expect(JSON.parse(String(write?.options.body))).toEqual({
      expectedRevision: 7,
      password: "new-fixture-password",
    });
  });

  it.each(["group", "user"])(
    "rejects an initial mixed %s read set and only edits explicitly reloaded state",
    async (kind) => {
      let revision = 7;
      let changed = false;
      const currentUser = { ...alice, active: false };
      const calls = api(
        context,
        (path) =>
          path.includes("/groups/")
            ? response({ id: "engineering", name: "Renamed", memberIds: [] })
            : response(currentUser),
        config,
        (path) => {
          if (path === "/api/admin/configuration")
            return response({ ...config, revision });
          if (path === "/api/admin/users") {
            if (kind === "user" && !changed) {
              changed = true;
              revision++;
              return response([context.user, { ...alice, role: "admin" }, bob]);
            }
            return response([context.user, currentUser, bob]);
          }
          if (path === "/api/admin/groups") {
            if (kind === "group" && !changed) {
              changed = true;
              revision++;
              return response([
                {
                  id: "engineering",
                  name: "Engineering",
                  memberIds: ["alice", "bob"],
                },
              ]);
            }
            return response([
              { id: "engineering", name: "Engineering", memberIds: [] },
            ]);
          }
        },
      );
      render(<AdminApp />);
      expect(
        await screen.findByText(
          "Administration changed while loading. Reload current state before editing.",
        ),
      ).toBeVisible();
      expect(
        screen.queryByRole("button", { name: /Edit group|Edit user/ }),
      ).not.toBeInTheDocument();
      expect(calls.filter((c) => c.options.method !== "GET")).toHaveLength(0);
      fireEvent.click(
        screen.getByRole("button", { name: "Reload current state" }),
      );
      await screen.findByRole("heading", { name: "MCP catalog" });
      fireEvent.click(
        screen.getByRole("link", {
          name: kind === "group" ? "Groups" : "Users",
        }),
      );
      fireEvent.click(
        await screen.findByRole("button", {
          name:
            kind === "group"
              ? "Edit group Engineering"
              : "Edit user alice (alice@example.test)",
        }),
      );
      if (kind === "group") {
        expect(
          screen.getByRole("checkbox", { name: "alice" }),
        ).not.toBeChecked();
        fireEvent.change(screen.getByLabelText("Group name"), {
          target: { value: "Renamed" },
        });
        fireEvent.click(screen.getByRole("button", { name: "Save group" }));
      } else {
        expect(
          screen.getByRole("checkbox", { name: "Active" }),
        ).not.toBeChecked();
        fireEvent.click(screen.getByRole("button", { name: "Save user" }));
      }
      await waitFor(() =>
        expect(calls.filter((c) => c.options.method === "PUT")).toHaveLength(1),
      );
      expect(
        JSON.parse(
          String(calls.find((c) => c.options.method === "PUT")?.options.body),
        ),
      ).toEqual(
        kind === "group"
          ? { expectedRevision: 8, name: "Renamed", memberIds: [] }
          : {
              expectedRevision: 8,
              name: "alice",
              email: "alice@example.test",
              role: "user",
              active: false,
            },
      );
    },
  );

  it.each(["race", "network"])(
    "keeps one confirmed mutation and blocks editing when follow-up reads encounter a %s",
    async (failure) => {
      let revision = 7;
      let confirmed = false;
      let raced = false;
      const calls = api(
        context,
        () => {
          confirmed = true;
          revision = 8;
          return response({
            id: "engineering",
            name: "Engineering",
            memberIds: ["alice"],
          });
        },
        config,
        (path) => {
          if (path === "/api/admin/configuration")
            return response({ ...config, revision });
          if (confirmed && path === "/api/admin/groups") {
            if (failure === "network")
              return response({ code: "offline", message: "Unavailable" }, 503);
            if (!raced) {
              raced = true;
              revision = 9;
            }
          }
        },
      );
      render(<AdminApp />);
      await screen.findByRole("heading", { name: "MCP catalog" });
      fireEvent.click(screen.getByRole("link", { name: "Groups" }));
      fireEvent.click(
        await screen.findByRole("button", { name: "Edit group Engineering" }),
      );
      fireEvent.click(screen.getByRole("checkbox", { name: "bob" }));
      fireEvent.click(screen.getByRole("button", { name: "Save group" }));
      expect(
        await screen.findByText(
          /The change was saved, but current state could not be loaded/,
        ),
      ).toBeVisible();
      expect(calls.filter((c) => c.options.method === "PUT")).toHaveLength(1);
      expect(
        screen.queryByRole("button", { name: "Edit group Engineering" }),
      ).not.toBeInTheDocument();
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    },
  );

  it.each(["policy", "connection", "edited"])(
    "preserves structured runtime and install commands during %s saves",
    async (mode) => {
      const configuration = structuredClone(config);
      const definition = configuration.mcps[0];
      definition.name = "N".repeat(200);
      const runtime = {
        program: "fixture",
        args: ["", "-c", 'print("first")\nprint("second")', "literal\\$value"],
        env: { ["K".repeat(129)]: "one\ntwo", EMPTY: "" },
        cwd: null,
      };
      const installation = {
        program: "install-fixture",
        args: ["", "literal\r\nvalue"],
        env: { INSTALL: "line\nnext" },
        cwd: null,
      };
      definition.transport = { kind: "stdio", command: runtime };
      definition.install = installation;
      const calls = api(
        context,
        () => response({ revision: 8 }),
        configuration,
      );
      render(<AdminApp />);
      fireEvent.click(
        await screen.findByRole("button", {
          name: `Manage ${definition.name}`,
        }),
      );
      expect(screen.getByLabelText("Name")).toHaveValue(definition.name);
      if (mode === "policy") {
        fireEvent.keyDown(screen.getByRole("tab", { name: "Tools" }), {
          key: "Enter",
        });
      } else if (mode === "connection") {
        fireEvent.change(screen.getByLabelText("Description"), {
          target: { value: "Updated description" },
        });
      } else {
        runtime.args = ["", "changed\nargument", "with spaces"];
        installation.env = { INSTALL: "changed\nvalue" };
        fireEvent.change(screen.getByLabelText("Runtime arguments"), {
          target: { value: JSON.stringify(runtime.args) },
        });
        fireEvent.change(screen.getByLabelText("Installation environment"), {
          target: { value: JSON.stringify(installation.env) },
        });
      }
      fireEvent.click(
        screen.getByRole("button", {
          name: mode === "policy" ? "Save policy" : "Save MCP",
        }),
      );
      await waitFor(() =>
        expect(
          calls.filter((c) => c.path === "/api/admin/mcps/mcp"),
        ).toHaveLength(1),
      );
      const saved = JSON.parse(
        String(
          calls.find((c) => c.path === "/api/admin/mcps/mcp")?.options.body,
        ),
      );
      expect(saved.definition.transport.command).toEqual(runtime);
      expect(saved.definition.install).toEqual(installation);
    },
  );

  it("rejects invalid structured arguments without sending a write", async () => {
    const calls = api();
    render(<AdminApp />);
    fireEvent.click(
      await screen.findByRole("button", { name: "Manage Local tools" }),
    );
    fireEvent.change(screen.getByLabelText("Runtime arguments"), {
      target: { value: '["valid", 3]' },
    });
    fireEvent.click(screen.getByRole("button", { name: "Save MCP" }));
    expect(
      await screen.findByText(/Arguments must be a bounded JSON array/),
    ).toBeVisible();
    expect(calls.filter((c) => c.options.method !== "GET")).toHaveLength(0);
  });

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
    fireEvent.change(await screen.findByLabelText("Email"), {
      target: { value: "alice@example.test" },
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
    expect(JSON.parse(String(sent?.options.body))).toEqual({
      email: "alice@example.test",
      password: "fixture-password",
    });
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
