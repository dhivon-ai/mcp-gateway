import type { ErrorCode } from "@/generated/ErrorCode";

const errorCodes: ErrorCode[] = [
  "unauthorized",
  "forbidden",
  "invalid_request",
  "conflict",
  "offline",
  "config_expired",
  "invalid_cache",
  "auth_required",
  "install_failed",
  "upstream_unavailable",
  "unsupported",
  "internal",
];

export class ApiFailure extends Error {
  constructor(
    public readonly code: ErrorCode,
    message: string,
    public readonly status: number,
  ) {
    super(message);
    this.name = "ApiFailure";
  }
}

function serverFailure(value: unknown, status: number): ApiFailure {
  if (
    typeof value === "object" &&
    value !== null &&
    "code" in value &&
    "message" in value &&
    typeof value.code === "string" &&
    errorCodes.some((code) => code === value.code) &&
    typeof value.message === "string" &&
    value.message.length <= 512
  ) {
    return new ApiFailure(value.code as ErrorCode, value.message, status);
  }
  return new ApiFailure(
    "internal",
    "The request could not be completed.",
    status,
  );
}

/** Callers must supply a runtime decoder; TypeScript alone does not validate JSON. */
export async function apiRequest<T>(
  path: string,
  decode: (value: unknown) => T,
  options: {
    method?: string;
    body?: unknown;
    csrfToken?: string;
    signal?: AbortSignal;
  } = {},
): Promise<T> {
  if (!path.startsWith("/api/") || path.includes("://") || path.includes("#")) {
    throw new ApiFailure(
      "invalid_request",
      "Only application API paths are permitted.",
      0,
    );
  }
  const method = options.method ?? "GET";
  if (!["GET", "HEAD"].includes(method) && !options.csrfToken) {
    throw new ApiFailure(
      "forbidden",
      "Refresh the page before making changes.",
      403,
    );
  }
  let response: Response;
  try {
    response = await fetch(path, {
      method,
      credentials: "same-origin",
      redirect: "error",
      signal: options.signal,
      headers: {
        "Content-Type": "application/json",
        ...(options.csrfToken ? { "X-CSRF-Token": options.csrfToken } : {}),
      },
      body:
        options.body === undefined ? undefined : JSON.stringify(options.body),
    });
  } catch {
    throw new ApiFailure(
      "offline",
      "Unable to reach the server. Changes require a connection.",
      0,
    );
  }
  let value: unknown;
  try {
    value = await response.json();
  } catch {
    throw new ApiFailure(
      "internal",
      "The server returned invalid data.",
      response.status,
    );
  }
  if (!response.ok) throw serverFailure(value, response.status);
  try {
    return decode(value);
  } catch {
    throw new ApiFailure(
      "internal",
      "The server returned invalid data.",
      response.status,
    );
  }
}
