import { movedGuides } from "./publication.ts";
import { createServer } from "node:http";
import { readFile, stat } from "node:fs/promises";
import { extname, join, resolve, sep } from "node:path";

const root = process.env.DHIVON_MCP_GATEWAY_BUILD_CACHE
  ? resolve(process.env.DHIVON_MCP_GATEWAY_BUILD_CACHE, "site")
  : resolve(import.meta.dirname, "dist");
const mime: Record<string, string> = {
  ".xml": "application/xml; charset=utf-8",
  ".html": "text/html; charset=utf-8",
  ".css": "text/css",
  ".svg": "image/svg+xml",
  ".png": "image/png",
  ".ico": "image/x-icon",
  ".ttf": "font/ttf",
  ".jpg": "image/jpeg",
  ".webp": "image/webp",
  ".txt": "text/plain; charset=utf-8",
};
await stat(join(root, "index.html"));
createServer(async (request, response) => {
  try {
    const url = new URL(request.url ?? "/", "http://127.0.0.1");
    const pathname = decodeURIComponent(url.pathname);
    const guide = /^\/docs\/([a-z0-9-]+)(?:\.html)?$/.exec(pathname)?.[1];
    if (guide && Object.hasOwn(movedGuides, guide)) {
      const [slug, fragment] = movedGuides[guide]!.split("#");
      response
        .writeHead(301, {
          Location:
            "/docs/" + slug + ".html" + (fragment ? "#" + fragment : ""),
        })
        .end();
      return;
    }
    const file = resolve(
      root,
      "." + (guide ? "/docs/" + guide + ".html" : pathname),
      pathname.endsWith("/") ? "index.html" : "",
    );
    if (!file.startsWith(root + sep)) {
      response.writeHead(403).end();
      return;
    }
    response.setHeader(
      "Content-Type",
      mime[extname(file)] ?? "application/octet-stream",
    );
    response.setHeader("X-Content-Type-Options", "nosniff");
    response.end(await readFile(file));
  } catch {
    response.writeHead(404).end("Page not found");
  }
}).listen(4173, "127.0.0.1", () =>
  console.log("Dhivon site preview http://127.0.0.1:4173"),
);
