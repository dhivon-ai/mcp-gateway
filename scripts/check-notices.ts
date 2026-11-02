/** Verify the emitted license bytes, rather than only the packaging recipe. */
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { basename, join, resolve } from "node:path";
import { parseArgs } from "node:util";

const root = resolve(import.meta.dirname, "..");
const notices = [
  { source: "LICENSE", file: "LICENSE", site: "LICENSE.txt" },
  { source: "NOTICE", file: "NOTICE", site: "NOTICE.txt" },
  {
    source: "assets/brand/fonts/OFL.txt",
    file: "licenses/Manrope-OFL.txt",
    site: "brand/fonts/OFL.txt",
  },
];
function run(command: string, args: string[]): Buffer {
  const result = spawnSync(command, args, {
    timeout: 60_000,
    maxBuffer: 4 * 1024 * 1024,
  });
  assert.equal(result.status, 0, `${command} failed while checking notices`);
  return result.stdout;
}
function verify(label: string, read: (file: string, site: string) => Buffer) {
  for (const { source, file, site } of notices) {
    assert.deepEqual(
      read(file, site),
      readFileSync(join(root, source)),
      `${label}: missing or altered ${source}`,
    );
  }
  console.log(`${label}: all project and Manrope notices match source bytes`);
}
export function verifyImageNotices(image: string) {
  verify(image, (file) =>
    run("docker", [
      "run",
      "--rm",
      "--network",
      "none",
      "--read-only",
      "--entrypoint",
      "/bin/cat",
      image,
      `/usr/share/doc/dhivon-mcp-gateway/${file}`,
    ]),
  );
}
if (import.meta.main) {
  const { values } = parseArgs({
    options: {
      archive: { type: "string" },
      image: { type: "string" },
      site: { type: "string" },
    },
  });
  assert(
    values.archive || values.image || values.site,
    "Supply --archive, --image or --site with a built artifact",
  );
  if (values.archive) {
    const archive = resolve(values.archive);
    const directory = basename(archive, ".tar.gz");
    verify(archive, (file) =>
      run("tar", ["-xOzf", archive, `${directory}/${file}`]),
    );
  }
  if (values.image) verifyImageNotices(values.image);
  if (values.site) {
    const directory = resolve(values.site);
    verify(directory, (_, site) => readFileSync(join(directory, site)));
    const page = readFileSync(join(directory, "docs/licensing.html"), "utf8");
    for (const path of [
      "/LICENSE.txt",
      "/NOTICE.txt",
      "/brand/fonts/OFL.txt",
    ]) {
      assert(
        page.includes(`href="${path}"`),
        `Missing published link: ${path}`,
      );
    }
    assert(page.includes("Copyright 2026 Dhivon AI"));
  }
}
