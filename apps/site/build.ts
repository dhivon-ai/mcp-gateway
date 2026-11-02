import { cp, mkdir, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { Marked } from "marked";

const root = resolve(import.meta.dirname, "../..");
const output = process.env.DHIVON_MCP_GATEWAY_BUILD_CACHE
  ? join(process.env.DHIVON_MCP_GATEWAY_BUILD_CACHE, "site")
  : join(root, "apps/site/dist");
await rm(output, { recursive: true, force: true });
await mkdir(join(output, "docs"), { recursive: true });
await cp(join(root, "assets/brand"), join(output, "brand"), {
  recursive: true,
});
await cp(join(root, "apps/site/style.css"), join(output, "style.css"));
await cp(join(root, "LICENSE"), join(output, "LICENSE.txt"));
await cp(join(root, "NOTICE"), join(output, "NOTICE.txt"));

const escape = (value: string) =>
  value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
const navigation = [
  ["index", "Documentation"],
  ["quickstart", "Quickstart"],
  ["architecture", "Why a gateway"],
  ["deployment", "Deploy and recover"],
  ["admin-ui", "Administration"],
  ["mac-install", "Mac setup"],
  ["agent-configuration", "Connect your agent"],
  ["integrations", "MCP integrations"],
  ["security", "Security"],
  ["configuration", "Configuration"],
  ["oauth", "OAuth"],
  ["troubleshooting", "Troubleshooting"],
  ["compatibility", "Compatibility"],
  ["contributing", "Contributing"],
  ["roadmap", "Roadmap"],
  ["licensing", "License and ownership"],
];
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
<a href="/docs/index.html">Documentation</a>
<a href="/docs/compatibility.html">Compatibility</a>
<a href="/docs/roadmap.html">Roadmap</a>
<a href="/docs/licensing.html">License and ownership</a>
</nav>
<p>Copyright 2026 Dhivon AI · <a href="/LICENSE.txt">Apache-2.0</a> · <a href="/NOTICE.txt">Notices</a></p>
</footer>`;
function page(title: string, description: string, body: string) {
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>${escape(title)} · Dhivon MCP Gateway</title>
<meta name="description" content="${escape(description)}">
<meta name="theme-color" content="#102C4C">
<link rel="icon" type="image/svg+xml" href="/brand/favicon.svg">
<link rel="apple-touch-icon" href="/brand/apple-touch-icon.png">
<link rel="stylesheet" href="/style.css">
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
  ),
);

async function renderDirectory(relative = "") {
  for (const entry of await readdir(join(root, "docs", relative), {
    withFileTypes: true,
  })) {
    const name = join(relative, entry.name);
    if (entry.isDirectory()) {
      await renderDirectory(name);
      continue;
    }
    const source = join(root, "docs", name);
    const target = join(output, "docs", name.replace(/\.md$/, ".html"));
    await mkdir(dirname(target), { recursive: true });
    if (!entry.name.endsWith(".md")) {
      if (/\.(png|svg|jpg|jpeg|webp)$/.test(entry.name))
        await cp(source, target);
      continue;
    }
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
    const title = markdown.match(/^# (.+)$/m)?.[1] ?? entry.name;
    const current = name.replace(/\.md$/, "");
    const sidebar = navigation
      .map(
        ([slug, label]) =>
          `<a href="/docs/${slug}.html"${current === slug ? ' aria-current="page"' : ""}>${label}</a>`,
      )
      .join("");
    const body = `<main id="main" class="docs-layout">
<aside>
<nav aria-label="Documentation">${sidebar}</nav>
</aside>
<article class="docs-article">${html}<p class="doc-source">Maintained in docs/${escape(name)} in the project repository.</p>
</article>
</main>`;
    await writeFile(
      target,
      page(title, title + " for Dhivon MCP Gateway.", body),
    );
  }
}
await renderDirectory();
console.log("Built public website at " + output);
