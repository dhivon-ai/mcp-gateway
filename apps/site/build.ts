import { createHash } from "node:crypto";
import { cp, mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { brandAssets, guides, movedGuides } from "./publication.ts";
import { Marked } from "marked";

const root = resolve(import.meta.dirname, "../..");
const output = process.env.DHIVON_MCP_GATEWAY_BUILD_CACHE
  ? join(process.env.DHIVON_MCP_GATEWAY_BUILD_CACHE, "site")
  : join(root, "apps/site/dist");
await rm(output, { recursive: true, force: true });
await mkdir(join(output, "docs"), { recursive: true });
for (const asset of brandAssets) {
  const destination = join(output, "brand", asset);
  await mkdir(dirname(destination), { recursive: true });
  await cp(join(root, "assets/brand", asset), destination);
}
const css = await readFile(join(root, "apps/site/style.css"));
const stylesheet = `style.${createHash("sha256").update(css).digest("hex").slice(0, 16)}.css`;
await writeFile(join(output, stylesheet), css);
await writeFile(join(output, "style.css"), css); // For previously served HTML.
await cp(join(root, "LICENSE"), join(output, "LICENSE.txt"));
await cp(join(root, "NOTICE"), join(output, "NOTICE.txt"));

const escape = (value: string) =>
  value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
const origin = "https://mcp.gateway.dhivon.ai";
const guidePath = (slug: string) =>
  slug === "index" ? "/docs/" : "/docs/" + slug;
const pagePaths = ["/", ...Object.keys(guides).map(guidePath)];
await writeFile(
  join(output, "sitemap.xml"),
  '<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n' +
    pagePaths
      .map((path) => `  <url><loc>${origin}${escape(path)}</loc></url>`)
      .join("\n") +
    "\n</urlset>\n",
);
await writeFile(
  join(output, "robots.txt"),
  `User-agent: *\nAllow: /\n\nSitemap: ${origin}/sitemap.xml\n`,
);

const navigation = [
  "index",
  "quickstart",
  "mac-install",
  "deployment",
  "agent-configuration",
  "troubleshooting",
] as const;
const header = `<header class="site-header">
<a class="identity" href="/" aria-label="Dhivon MCP Gateway home">
<img src="/brand/dhivon.svg" alt="dhivon" width="142" height="40">
<span>MCP Gateway</span>
</a>
<nav aria-label="Main navigation">
<a href="/#why">Why a gateway</a>
<a href="/docs/index.html">Documentation</a>
<a href="/docs/roadmap.html">Roadmap</a>
<a class="button" href="/docs/quickstart.html">Get started</a>
</nav>
</header>`;
const footer = `<footer>
<a class="identity" href="/">
<img src="/brand/dhivon.svg" alt="dhivon" width="142" height="40">
<span>MCP Gateway, a product of Dhivon AI.</span>
</a>
<nav aria-label="Footer">
<a href="https://dhivon.ai/sitemap">Dhivon sites</a>
<a href="/docs/index.html">Documentation</a>
<a href="/docs/compatibility.html">Compatibility</a>
<a href="/docs/roadmap.html">Roadmap</a>
<a href="/docs/licensing.html">License and ownership</a>
</nav>
<p>Copyright 2026 Dhivon AI · <a href="/LICENSE.txt">Apache-2.0</a> · <a href="/NOTICE.txt">Notices</a></p>
</footer>`;
function page(title: string, description: string, body: string, path?: string) {
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>${escape(title)} · Dhivon MCP Gateway</title>
<meta name="description" content="${escape(description)}">
${path ? `<link rel="canonical" href="${origin}${escape(path)}">` : '<meta name="robots" content="noindex">'}
<meta name="theme-color" content="#102C4C">
<link rel="icon" type="image/svg+xml" href="/brand/favicon.svg">
<link rel="apple-touch-icon" href="/brand/apple-touch-icon.png">
<link rel="stylesheet" href="/${stylesheet}">
</head>
<body>
<a class="skip" href="#main">Skip to content</a>${header}${body}${footer}</body>
</html>`;
}
const diagram = `<div class="diagram" role="img" aria-label="The control plane sends policy to the local gateway. A coding agent uses one STDIO connection to that gateway. The gateway calls local STDIO tools on the Mac or remote Streamable HTTP tools at their service.">
<div class="diagram-control">
<strong>Your control plane</strong>
<small>Identity · assignments · tool policy</small>
</div>
<div class="policy-line">Access policy ↓</div>
<div class="diagram-flow">
<div class="diagram-node">Coding agent<small>One STDIO entry</small>
</div>
<span class="arrow" aria-hidden="true">→</span>
<div class="diagram-node gateway">
<img src="/brand/logo.svg" width="54" height="54" alt="">
<strong>Local gateway</strong>
<small>Developer opt-in + policy checks</small>
</div>
<span class="arrow" aria-hidden="true">→</span>
<div class="upstreams">
<div class="diagram-node">
<strong>Local STDIO tools</strong>
<small>Run on the developer’s Mac</small>
</div>
<div class="diagram-node">
<strong>Remote HTTP tools</strong>
<small>Run at the upstream service</small>
</div>
</div>
</div>
</div>`;
const steps = [
  [
    "01",
    "Grant",
    "An admin assigns the Engineering team an echo fixture MCP and permits its echo tool.",
  ],
  [
    "02",
    "Enable",
    "A developer signs in on their Mac and explicitly enables the assigned MCP.",
  ],
  [
    "03",
    "Call",
    "The agent discovers and calls the permitted echo tool through one STDIO entry.",
  ],
  [
    "04",
    "Disable",
    "After confirmed disablement, the gateway denies subsequent calls to its previous alias.",
  ],
];
const roles = [
  [
    "01",
    "Operators",
    "Deploy the control plane, bootstrap an admin and keep keys safe.",
    "Deployment guide",
    "deployment",
  ],
  [
    "02",
    "Administrators",
    "Create a catalog, assign access and set tool policies.",
    "Admin guide",
    "admin-ui",
  ],
  [
    "03",
    "Developers",
    "Install the Mac gateway, enable an MCP and connect your agent.",
    "First tool call",
    "quickstart",
  ],
];
const home = `<main id="main">
<section class="hero">
<div class="hero-copy">
<h1>One connection.<br>Clear control.</h1>
<p>Dhivon MCP Gateway connects coding agents to the tools your team permits. Admins set access. Developers choose what to enable.</p>
<div class="actions">
<a class="button" href="/docs/quickstart.html">Get started</a>
<a class="text-link" href="#why">How it works <span aria-hidden="true">↓</span>
</a>
</div>
<p class="release-note">Self-hosted control plane · Apple Silicon local gateway</p>
</div>${diagram}</section>
<section class="why" id="why">
<div class="why-intro">
<h2>Give every tool connection an owner.</h2>
<div>
<p>Adding each upstream to every agent spreads configuration and makes access difficult to follow. Dhivon puts the permission decision in one place and keeps execution on the developer’s machine or the chosen remote server.</p>
<p>The control plane owns identity, assignments and tool policy. The local gateway checks policy, developer opt-in and credentials before each call.</p>
<a class="text-link" href="/docs/architecture.html">Explore the architecture →</a>
</div>
</div>
<div class="steps">${steps.map(([n, t, d]) => `<div><span class="step-number">${n}</span><h3>${t}</h3><p>${d}</p></div>`).join("")}</div>
<a class="text-link" href="/docs/quickstart.html">Follow the complete echo walkthrough →</a>
</section>
<section class="roles">
<h2>Start with your role.</h2>${roles.map(([n, t, d, l, p]) => `<div class="role-row"><span class="role-number">${n}</span><h3>${t}</h3><p>${d}</p><a class="text-link" href="/docs/${p}.html">${l} →</a></div>`).join("")}</section>
<section class="roadmap-band">
<h2>What works. What comes next.</h2>
<p>The current release supports tools over local STDIO and Streamable HTTP, with none, bearer or supported public-client OAuth authentication.</p>
<a class="button outline" href="/docs/roadmap.html">Read the roadmap →</a>
<p class="release-note">The local archive is unsigned and unpublished. Read the <a href="/docs/compatibility.html">compatibility guide</a> for tested platforms and release limits.</p>
</section>
</main>`;
await writeFile(
  join(output, "index.html"),
  page(
    "One connection. Clear control.",
    "Self-hosted MCP access for teams. Administrators set policy, developers opt in, and agents connect through one local gateway.",
    home,
    "/",
  ),
);

await writeFile(
  join(output, "404.html"),
  page(
    "Page not found",
    "This page could not be found.",
    '<main id="main"><section class="why"><h1>Page not found</h1><p><a href="/docs/index.html">Browse the documentation</a></p></section></main>',
  ),
);

for (const current of Object.keys(guides)) {
  const name = current + ".md";
  const source = join(root, "docs", name);
  const target = join(output, "docs", current + ".html");
  const markdown = await readFile(source, "utf8");
  const slugs = new Map<string, number>();
  const parser = new Marked({
    gfm: true,
    renderer: {
      heading({ tokens, depth }) {
        const text = this.parser.parseInline(tokens);
        const base = text
          .replace(/<[^>]*>/g, "")
          .toLowerCase()
          .replace(/[^\p{L}\p{N}\s_-]/gu, "")
          .trim()
          .replace(/\s/g, "-");
        const count = slugs.get(base) ?? 0;
        slugs.set(base, count + 1);
        const id = count ? base + "-" + count : base;
        return `<h${depth} id="${escape(id)}">${text}</h${depth}>\n`;
      },
      link({ href, title, tokens }) {
        const notices: Record<string, string> = {
          "../LICENSE": "/LICENSE.txt",
          "../NOTICE": "/NOTICE.txt",
          "../assets/brand/fonts/OFL.txt": "/brand/fonts/OFL.txt",
        };
        const url = notices[href] ?? href.replace(/\.md(?=#|$)/, ".html");
        if (/^(javascript|data):/i.test(url))
          throw new Error("Unsafe documentation URL");
        return `<a href="${escape(url)}"${title ? ' title="' + escape(title) + '"' : ""}>${this.parser.parseInline(tokens)}</a>`;
      },
    },
  });
  // Keep Marked's built-in table renderer and wrap its output for narrow screens.
  const html = (await parser.parse(markdown))
    .replaceAll("<table>", '<div class="table-scroll"><table>')
    .replaceAll("</table>", "</table></div>");
  const title = markdown.match(/^# (.+)$/m)?.[1] ?? name;
  const sidebar = navigation
    .map(
      (slug) =>
        `<a href="/docs/${slug}.html"${current === slug ? ' aria-current="page"' : ""}>${guides[slug]}</a>`,
    )
    .join("");
  const directory =
    current === "index"
      ? '<h2 id="all-guides">All guides</h2><ul>' +
        Object.entries(guides)
          .filter(([slug]) => slug !== "index")
          .map(
            ([slug, label]) =>
              `<li><a href="/docs/${slug}.html">${escape(label)}</a></li>`,
          )
          .join("") +
        "</ul>"
      : "";
  const body = `<main id="main" class="docs-layout">
<article class="docs-article">${html}${directory}</article>
<aside><nav aria-label="Documentation">${sidebar}</nav></aside>
</main>`;
  await writeFile(
    target,
    page(title, title + " for Dhivon MCP Gateway.", body, guidePath(current)),
  );
}
const redirects = Object.entries(movedGuides).flatMap(([old, destination]) =>
  [`/docs/${old}`, `/docs/${old}.html`].map(
    (path) => `${path} /docs/${destination} 301`,
  ),
);
await writeFile(join(output, "_redirects"), redirects.join("\n") + "\n");

// Keep public guides and redirects on Pages' static path. For other doc URLs,
// the 404 Function must run before Pages can return a preserved, removed asset.
const staticDocs = [
  "/docs/",
  ...Object.keys(guides).flatMap((slug) =>
    ["", ".html", "/"].map((suffix) => `/docs/${slug}${suffix}`),
  ),
  ...Object.keys(movedGuides).flatMap((slug) => [
    `/docs/${slug}`,
    `/docs/${slug}.html`,
  ]),
];
const routes = { version: 1, include: ["/docs/*"], exclude: staticDocs };
if (routes.include.length + routes.exclude.length > 100)
  throw new Error("Documentation routes exceed the Cloudflare Pages limit.");
await writeFile(join(output, "_routes.json"), JSON.stringify(routes, null, 2));
console.log("Built public website at " + output);
