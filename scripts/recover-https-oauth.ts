/** Retry exact owned cleanup; no production project/URL/credential arguments. */
import assert from "node:assert/strict";
import { Rehearsal } from "./rehearsal-lifecycle.ts";
const cache = process.env.DHIVON_MCP_GATEWAY_BUILD_CACHE;
assert(
  cache && process.argv.length === 3,
  "Set the original owned build cache and supply its rehearsal directory",
);
const rehearsal = Rehearsal.recover(cache, process.argv[2]);
rehearsal.supervise();
try {
  const clean = await rehearsal.cleanup();
  console.log(
    `Cleanup ${clean ? "verified" : "incomplete"}; retained ${rehearsal.directory}/result.json. Remove approved temporary trust separately.`,
  );
  process.exitCode = clean && rehearsal.journal.cleanup.diagnostics?.ok ? 0 : 1;
} finally {
  rehearsal.close();
}
