// Keep the archive usable offline. Follow links only into explicitly public guides.
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { guides } from "../apps/site/publication.ts";

const output = process.argv[2];
if (!output) throw new Error("Supply the archive staging directory");
mkdirSync(join(output, "guides"));
const entries = new Map([
  ["mac-install", "README.md"],
  ["compatibility", "COMPATIBILITY.md"],
]);
const pending = new Set(entries.keys());
const localPath = (slug: string) => entries.get(slug) ?? `guides/${slug}.md`;

for (const slug of pending) {
  const source = readFileSync(
    join(import.meta.dirname, "../docs", slug + ".md"),
    "utf8",
  );
  const markdown = source.replace(
    /\]\(([a-z0-9-]+)\.md(#[^()\s]+)?\)/g,
    (_, target: string, fragment = "") => {
      if (!Object.hasOwn(guides, target))
        throw new Error(
          `${slug} links to a guide outside the public catalog: ${target}`,
        );
      pending.add(target);
      const path = relative(dirname(localPath(slug)), localPath(target));
      return `](${path}${fragment})`;
    },
  );
  writeFileSync(join(output, localPath(slug)), markdown);
}
