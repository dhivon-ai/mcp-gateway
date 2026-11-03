// Check the emitted site, including new files that must stay out of publication.
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
  cp,
  mkdir,
  mkdtemp,
  readFile,
  readdir,
  rm,
  symlink,
  writeFile,
} from "node:fs/promises";
import { join, relative, resolve } from "node:path";
import { tmpdir } from "node:os";
import { brandAssets, guides, movedGuides } from "./publication.ts";

const root = resolve(import.meta.dirname, "../..");
const scratch = await mkdtemp(
  join(process.env.DHIVON_MCP_GATEWAY_BUILD_CACHE || tmpdir(), "site-check-"),
);

try {
  const src = join(scratch, "source");
  const site = join(scratch, "build", "site");
  await mkdir(join(src, "apps/site"), { recursive: true });
  for (const file of [
    "LICENSE",
    "NOTICE",
    "package.json",
    "docs",
    "assets/brand",
    "apps/site/build.ts",
    "apps/site/style.css",
    "apps/site/publication.ts",
  ]) {
    await cp(join(root, file), join(src, file), { recursive: true });
  }
  await symlink(
    join(root, "apps/site/node_modules"),
    join(src, "apps/site/node_modules"),
    "dir",
  );
  for (const file of [
    "docs/unlisted.md",
    "docs/unlisted.png",
    "assets/brand/unlisted.txt",
  ]) {
    await writeFile(join(src, file), "PRIVATE_FIXTURE_NOT_FOR_PUBLICATION");
  }
  const build = spawnSync(process.execPath, [join(src, "apps/site/build.ts")], {
    env: {
      ...process.env,
      DHIVON_MCP_GATEWAY_BUILD_CACHE: join(scratch, "build"),
    },
    encoding: "utf8",
  });
  assert.equal(build.status, 0, build.stderr);

  const expected = new Set([
    "index.html",
    "404.html",
    "_redirects",
    "_routes.json",
    "style.css",
    "LICENSE.txt",
    "NOTICE.txt",
    ...Object.keys(guides).map((slug) => "docs/" + slug + ".html"),
    ...brandAssets.map((asset) => "brand/" + asset),
  ]);
  const files = await readdir(site, { recursive: true, withFileTypes: true });
  const actual = new Set(
    files
      .filter((f) => f.isFile())
      .map((f) => relative(site, join(f.parentPath, f.name))),
  );
  assert.deepEqual(actual, expected, "Unexpected or missing public files");

  const routes = JSON.parse(await readFile(join(site, "_routes.json"), "utf8"));
  assert.deepEqual(routes.include, ["/docs/*"]);
  // Every exception bypasses the 404 Function. It must resolve to a file
  // published in this build or a deliberate redirect.
  for (const path of routes.exclude) {
    assert.ok(path.startsWith("/docs/"));
    const slug = path.slice(6).replace(/(?:\.html|\/)$/, "") || "index";
    assert.ok(
      expected.has("docs/" + slug + ".html") ||
        Object.hasOwn(movedGuides, slug),
      path,
    );
  }

  for (const file of expected) {
    if (!/\.(html|css)$/.test(file)) continue;
    const content = await readFile(join(site, file), "utf8");
    assert.doesNotMatch(
      content,
      /PRIVATE_FIXTURE_NOT_FOR_PUBLICATION|\b(?:BUG|AC|FR|NFR)-\d+\b|\.cadre\//i,
      file,
    );
    const links = file.endsWith(".html")
      ? /(?:href|src)="([^"]+)"/g
      : /url\(["']?([^)'"]+)/g;
    for (const match of content.matchAll(links)) {
      const url = new URL(match[1]!, "https://preview.invalid/" + file);
      if (url.origin !== "https://preview.invalid") continue;
      const target = decodeURIComponent(url.pathname).slice(1) || "index.html";
      assert.ok(expected.has(target), file + " links to missing " + target);
      if (url.hash) {
        const page = await readFile(join(site, target), "utf8");
        assert.ok(
          page.includes('id="' + decodeURIComponent(url.hash.slice(1)) + '"'),
          file + " broken fragment " + url.href,
        );
      }
    }
  }
  const redirects = await readFile(join(site, "_redirects"), "utf8");
  for (const [old, target] of Object.entries(movedGuides)) {
    const [slug, fragment] = target.split("#");
    assert.ok(expected.has("docs/" + slug + ".html"), old);
    if (fragment)
      assert.ok(
        (await readFile(join(site, "docs", slug + ".html"), "utf8")).includes(
          'id="' + fragment + '"',
        ),
      );
    for (const suffix of ["", ".html"])
      assert.ok(
        redirects.includes(
          "/docs/" + old + suffix + " /docs/" + target + " 301\n",
        ),
      );
  }
  console.log(
    "Checked " +
      expected.size +
      " public files, links, anchors, redirects and unpublished fixtures.",
  );
} finally {
  await rm(scratch, { recursive: true, force: true });
}
