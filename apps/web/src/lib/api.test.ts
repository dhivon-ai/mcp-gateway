import { afterEach, describe, expect, it, vi } from "vitest";
import { apiRequest, ApiFailure } from "./api";

afterEach(() => vi.unstubAllGlobals());
const status = (value: unknown): string => {
  if (
    typeof value !== "object" ||
    value === null ||
    !("status" in value) ||
    value.status !== "ok"
  )
    throw new Error("bad contract");
  return value.status;
};

describe("API boundary", () => {
  it("decodes responses and keeps session transport same-origin", async () => {
    const fetch = vi.fn().mockResolvedValue(new Response('{"status":"ok"}'));
    vi.stubGlobal("fetch", fetch);
    expect(await apiRequest("/api/context", status)).toBe("ok");
    expect(fetch).toHaveBeenCalledWith(
      "/api/context",
      expect.objectContaining({
        credentials: "same-origin",
        redirect: "error",
      }),
    );
  });
  it("requires CSRF and rejects third-party credential-bearing requests", async () => {
    const fetch = vi.fn();
    vi.stubGlobal("fetch", fetch);
    await expect(
      apiRequest("/api/settings", status, { method: "PUT" }),
    ).rejects.toMatchObject({ code: "forbidden" });
    await expect(
      apiRequest("https://elsewhere.test/api/", status),
    ).rejects.toMatchObject({ code: "invalid_request" });
    expect(fetch).not.toHaveBeenCalled();
  });
  it("rejects invalid JSON contracts without displaying raw payloads", async () => {
    vi.stubGlobal(
      "fetch",
      vi
        .fn()
        .mockResolvedValue(new Response('{"token":"sensitive-fixture-value"}')),
    );
    await expect(apiRequest("/api/context", status)).rejects.toMatchObject({
      message: "The server returned invalid data.",
    });
  });
  it("uses safe errors for unrecognized error shapes and outages", async () => {
    const fetch = vi
      .fn()
      .mockResolvedValueOnce(
        new Response('{"message":"sensitive-fixture-value"}', { status: 500 }),
      )
      .mockRejectedValueOnce(new Error("raw transport details"));
    vi.stubGlobal("fetch", fetch);
    await expect(apiRequest("/api/context", status)).rejects.toEqual(
      new ApiFailure("internal", "The request could not be completed.", 500),
    );
    await expect(apiRequest("/api/context", status)).rejects.toMatchObject({
      code: "offline",
    });
  });
});
