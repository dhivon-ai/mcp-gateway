import "@testing-library/jest-dom/vitest";
import { StrictMode } from "react";
import { beforeEach, afterEach, describe, it, expect, vi } from "vitest";
import {
  render,
  screen,
  fireEvent,
  waitFor,
  cleanup,
  within,
  act,
} from "@testing-library/react";
import { LocalApp } from "./app";
import { remaining, type LocalContext } from "./contracts";
import type { LocalCatalog } from "@/generated/LocalCatalog";

const generation = "00000000-0000-4000-8000-000000000001";
function initial(): LocalContext {
  const now = Math.floor(Date.now() / 1000);
  return {
    mode: "local",
    csrfToken: "local-csrf",
    serverUrl: "https://central.example",
    status: {
      identity: {
        state: "authenticated",
        generation,
        username: "alice",
        sessionExpiresAt: now + 43200,
      },
      configuration: {
        state: "valid",
        revision: 7,
        validatedAt: now,
        expiresAt: now + 3600,
        remainingSeconds: 3600,
      },
      online: true,
      lastSyncError: null,
    },
  };
}
function catalog(): LocalCatalog {
  return {
    generation,
    revision: 7,
    expiresAt: Math.floor(Date.now() / 1000) + 3600,
    mcps: [
      {
        id: "local",
        name: "Local tools",
        description: "",
        transport: "stdio",
        auth: "none",
        enabled: false,
        secret: null,
        installationAvailable: true,
        installed: false,
        availability: "disabled",
        authRequired: false,
        toolPolicy: { mode: "allowlist", tools: ["echo"] },
      },
    ],
  };
}
function response(value: unknown, status = 200) {
  return new Response(value === null ? null : JSON.stringify(value), {
    status,
    headers: { "content-type": "application/json" },
  });
}
function api(
  context = initial(),
  config = catalog(),
  mutation?: (
    path: string,
    options: RequestInit,
  ) => Response | Promise<Response>,
) {
  const calls: { path: string; options: RequestInit }[] = [];
  vi.stubGlobal(
    "fetch",
    vi.fn(async (path: string, options: RequestInit) => {
      calls.push({ path, options });
      if (options.method !== "GET" && options.method !== undefined)
        return (
          mutation?.(path, options) ??
          response({ code: "internal", message: "Unexpected write" }, 500)
        );
      if (path === "/api/context") return response(context);
      if (path === "/api/catalog") return response(config);
      if (path === "/api/secrets")
        return response([
          { id: "owned-key", key: "work-token", version: 1, updatedAt: 1 },
        ]);
      if (path.endsWith("/status"))
        return response({
          connected: false,
          version: null,
          expiresAt: null,
          readiness: "disconnected",
        });
      return response({ code: "internal", message: "Unexpected read" }, 500);
    }),
  );
  return calls;
}
beforeEach(() => {
  Object.defineProperty(HTMLElement.prototype, "scrollIntoView", {
    configurable: true,
    value: vi.fn(),
  });
  vi.stubGlobal(
    "ResizeObserver",
    class {
      observe() {}
      unobserve() {}
      disconnect() {}
    },
  );
  vi.stubGlobal(
    "open",
    vi.fn(() => null),
  );
});
afterEach(() => {
  cleanup();
  vi.unstubAllGlobals();
  vi.restoreAllMocks();
  window.location.hash = "";
});
describe("Local management journeys", () => {
  it("ignores a late context response from the previous identity", async () => {
    const alice = initial(),
      bob = initial();
    bob.status.identity.username = "bob";
    bob.status.identity.generation = "00000000-0000-4000-8000-000000000002";
    let resolveOld: (value: Response) => void = () => {};
    let contexts = 0;
    vi.stubGlobal(
      "fetch",
      vi.fn(async (path: string) => {
        if (path === "/api/context") {
          if (++contexts === 1)
            return new Promise<Response>((done) => {
              resolveOld = done;
            });
          return response(bob);
        }
        return response({
          ...catalog(),
          generation: bob.status.identity.generation,
        });
      }),
    );
    render(<LocalApp />);
    await screen.findByText("Loading local configuration");
    fireEvent(window, new Event("focus"));
    await screen.findByText("bob");
    await act(async () => {
      resolveOld(response(alice));
    });
    expect(screen.getByText("bob")).toBeVisible();
    expect(screen.queryByText("alice")).not.toBeInTheDocument();
  });
  it("clears rejected passwords and sends CSRF without browser persistence", async () => {
    const c = initial();
    c.status.identity.state = "login_required";
    c.status.identity.generation = null;
    const calls = api(c, catalog(), () =>
      response({ code: "unauthorized", message: "Login is required" }, 401),
    );
    render(<LocalApp />);
    fireEvent.change(await screen.findByLabelText("Username"), {
      target: { value: "alice" },
    });
    fireEvent.change(screen.getByLabelText("Password"), {
      target: { value: "fixture-password" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Sign in" }));
    await screen.findByText("Login is required");
    expect(screen.getByLabelText("Password")).toHaveValue("");
    expect(
      calls.find((c) => c.path === "/api/auth/login")?.options.headers,
    ).toMatchObject({ "X-CSRF-Token": "local-csrf" });
    expect(localStorage.length).toBe(0);
  });
  it("loads under StrictMode and waits for a confirmed settings response before changing enablement", async () => {
    const config = catalog();
    let resolveWrite: (value: Response) => void = () => {};
    const calls = api(
      initial(),
      config,
      () =>
        new Promise<Response>((r) => {
          resolveWrite = r;
        }),
    );
    render(
      <StrictMode>
        <LocalApp />
      </StrictMode>,
    );
    fireEvent.click(
      (await screen.findAllByRole("button", { name: "Manage" }))[0],
    );
    fireEvent.click(screen.getByRole("button", { name: "Enable MCP" }));
    await waitFor(() =>
      expect(calls.some((c) => c.path === "/api/settings/local")).toBe(true),
    );
    expect(screen.getByText("Disabled", { selector: "p" })).toBeVisible();
    resolveWrite(
      response({
        ...config,
        revision: 8,
        mcps: config.mcps.map((m) => ({ ...m, enabled: true })),
      }),
    );
    await screen.findByText("Server confirmed the current settings.");
    expect(screen.getByText("Enabled")).toBeVisible();
    expect(
      JSON.parse(
        String(
          calls.find((c) => c.path === "/api/settings/local")?.options.body,
        ),
      ),
    ).toEqual({
      expectedGeneration: generation,
      input: { expectedRevision: 7, enabled: true, secretId: null },
    });
  });
  it("requires explicit reload after a revision conflict without repeating a mutation", async () => {
    const calls = api(initial(), catalog(), () =>
      response(
        { code: "conflict", message: "Configuration changed", revision: 8 },
        409,
      ),
    );
    render(<LocalApp />);
    fireEvent.click(
      (await screen.findAllByRole("button", { name: "Manage" }))[0],
    );
    fireEvent.click(screen.getByRole("button", { name: "Enable MCP" }));
    await screen.findByText("Changes need review");
    expect(screen.getByRole("button", { name: "Enable MCP" })).toBeDisabled();
    expect(calls.filter((c) => c.path === "/api/settings/local")).toHaveLength(
      1,
    );
    fireEvent.click(
      screen.getByRole("button", { name: "Reload current state" }),
    );
    await waitFor(() =>
      expect(screen.queryByRole("dialog")).not.toBeInTheDocument(),
    );
    expect(calls.filter((c) => c.path === "/api/settings/local")).toHaveLength(
      1,
    );
  });
  it("selects only current-user key metadata for bearer enablement", async () => {
    const config = catalog();
    config.mcps[0] = {
      ...config.mcps[0],
      auth: "bearer",
      transport: "http",
      installationAvailable: false,
    };
    const calls = api(initial(), config, () =>
      response({
        ...config,
        revision: 8,
        mcps: config.mcps.map((m) => ({ ...m, enabled: true })),
      }),
    );
    render(<LocalApp />);
    fireEvent.click(
      (await screen.findAllByRole("button", { name: "Manage" }))[0],
    );
    const select = await screen.findByRole("combobox", { name: "Secret key" });
    await waitFor(() => expect(select).toBeEnabled());
    expect(screen.getByRole("button", { name: "Enable MCP" })).toBeDisabled();
    fireEvent.keyDown(select, { key: "ArrowDown" });
    fireEvent.click(await screen.findByRole("option", { name: "work-token" }));
    fireEvent.click(screen.getByRole("button", { name: "Enable MCP" }));
    await screen.findByText("Server confirmed the current settings.");
    const call = calls.find((c) => c.path === "/api/settings/local");
    expect(JSON.parse(String(call?.options.body)).input.secretId).toBe(
      "owned-key",
    );
    expect(document.body.textContent).not.toContain("sensitive-token");
  });
  it("clears write-only values after failed secret creation and does not load a value for replacement", async () => {
    window.location.hash = "secrets";
    const calls = api(initial(), catalog(), () =>
      response(
        { code: "conflict", message: "Secret changed", revision: 8 },
        409,
      ),
    );
    render(<LocalApp />);
    fireEvent.click(await screen.findByRole("button", { name: "Add secret" }));
    const dialog = screen.getByRole("dialog");
    fireEvent.change(within(dialog).getByLabelText("Key"), {
      target: { value: "new-key" },
    });
    fireEvent.change(within(dialog).getByLabelText("Value"), {
      target: { value: "sensitive-token" },
    });
    fireEvent.submit(
      within(dialog)
        .getByRole("button", { name: "Save secret" })
        .closest("form")!,
    );
    await screen.findByText("Secret changed");
    expect(within(dialog).getByLabelText("Value")).toHaveValue("");
    expect(calls.filter((c) => c.options.method === "POST")).toHaveLength(1);
    expect(calls.every((c) => !c.path.includes("/value"))).toBe(true);
  });
  it("reports installation failure without enabled success and never starts while browsing", async () => {
    const calls = api(initial(), catalog(), () =>
      response(
        {
          id: "job",
          mcpId: "local",
          generation,
          state: "failed",
          startedAt: 1,
          completedAt: 2,
          elapsedSeconds: 1,
          exitCode: 1,
          message: "Fixed private outcome",
        },
        202,
      ),
    );
    render(<LocalApp />);
    fireEvent.click(await screen.findByRole("button", { name: "Install" }));
    expect(calls.filter((c) => c.options.method === "POST")).toHaveLength(0);
    fireEvent.click(screen.getByRole("button", { name: "Run installation" }));
    await screen.findByText("Install failed");
    expect(calls.some((c) => c.path.startsWith("/api/settings/"))).toBe(false);
    expect(
      screen.queryByText("Server confirmed the current settings."),
    ).not.toBeInTheDocument();
  });
  it("keeps offline configuration read-only and omits vault reads", async () => {
    const c = initial();
    c.status.online = false;
    const calls = api(c);
    render(<LocalApp />);
    await screen.findByText("Offline · Read-only");
    fireEvent.click(
      (await screen.findAllByRole("button", { name: "Manage" }))[0],
    );
    expect(screen.getByRole("button", { name: "Enable MCP" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "Install" })).toBeDisabled();
    expect(calls.every((c) => c.options.method === "GET")).toBe(true);
  });
  it("shows no usable catalog at native expiry, including exact equality", async () => {
    const c = initial();
    const now = Math.floor(Date.now() / 1000) * 1000;
    c.status.configuration.expiresAt = now / 1000;
    c.status.configuration.remainingSeconds = 3600;
    expect(remaining(c, now, now)).toBe(0);
    c.status.configuration.expiresAt = now / 1000 + 3600;
    expect(remaining(c, now, now + 3599000)).toBe(1);
    expect(remaining(c, now, now + 3600000)).toBe(0);
    c.status.configuration.state = "expired";
    const calls = api(c);
    render(<LocalApp />);
    await screen.findByText("Configuration unavailable");
    expect(calls.some((c) => c.path === "/api/catalog")).toBe(false);
    expect(
      screen.queryByRole("button", { name: "Manage" }),
    ).not.toBeInTheDocument();
  });
  it("starts OAuth only on an explicit action and cancellation never enables the MCP", async () => {
    const config = catalog();
    config.mcps[0] = {
      ...config.mcps[0],
      auth: "oauth",
      transport: "http",
      installationAvailable: false,
    };
    const calls = api(initial(), config, (path) =>
      path.endsWith("/connect")
        ? response({
            flowId: "flow",
            authorizationUrl: "https://provider.example/authorize?state=opaque",
          })
        : response({ cancelled: true }),
    );
    render(<LocalApp />);
    fireEvent.click(
      (await screen.findAllByRole("button", { name: "Manage" }))[0],
    );
    await screen.findByText("Connect OAuth to continue");
    expect(calls.filter((c) => c.options.method === "POST")).toHaveLength(0);
    fireEvent.click(screen.getByRole("button", { name: "Connect OAuth" }));
    fireEvent.click(
      await screen.findByRole("button", { name: "Cancel authorization" }),
    );
    await screen.findByText(/Authorization cancelled/);
    expect(calls.some((c) => c.path === "/api/oauth/flows/flow/cancel")).toBe(
      true,
    );
    expect(calls.some((c) => c.path.startsWith("/api/settings/"))).toBe(false);
  });
});
