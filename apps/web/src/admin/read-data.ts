import { apiRequest, ApiFailure } from "@/lib/api";
import * as decode from "@/lib/decode";
import type { AdminData } from "./types";

/** All identity/group writes bump the global revision. Bracket their reads before editing. */
export async function readAdminData(signal?: AbortSignal): Promise<AdminData> {
  const before = await apiRequest(
    "/api/admin/configuration",
    decode.configuration,
    { signal },
  );
  const [users, groups] = await Promise.all([
    apiRequest("/api/admin/users", (v) => decode.array(v, decode.user), {
      signal,
    }),
    apiRequest("/api/admin/groups", (v) => decode.array(v, decode.group), {
      signal,
    }),
  ]);
  const config = await apiRequest(
    "/api/admin/configuration",
    decode.configuration,
    { signal },
  );
  if (before.revision !== config.revision) {
    throw new ApiFailure(
      "conflict",
      "Administration changed while loading. Reload current state before editing.",
      409,
      config.revision,
    );
  }
  return { config, users, groups };
}
