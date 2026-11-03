// Only listed guides and brand assets belong in the public distribution.
export const guides = {
  index: "Guide directory",
  quickstart: "Get started",
  "mac-install": "Connect your Mac",
  deployment: "Host a gateway",
  "agent-configuration": "Connect your agent",
  "admin-ui": "Manage team access",
  "local-ui": "Manage your tools",
  integrations: "Add an MCP server",
  configuration: "Tool access rules",
  oauth: "Provider sign-in",
  architecture: "How the gateway works",
  security: "Security and offline access",
  "endpoint-identity": "Mac recovery",
  vault: "Secrets and keys",
  operations: "Backup and restore",
  compatibility: "Supported platforms",
  checksums: "Verify an archive",
  troubleshooting: "Troubleshooting",
  roadmap: "Roadmap",
  licensing: "License and notices",
} as const;

export const brandAssets = [
  "dhivon.svg",
  "logo.svg",
  "favicon.svg",
  "apple-touch-icon.png",
  "theme.css",
  "fonts/Manrope-wght.ttf",
  "fonts/OFL.txt",
];

// Moved guides point to the corresponding public task. Internal reports return 404.
export const movedGuides: Record<string, string> = {
  "remote-credentials": "vault",
  "endpoint-cache": "security#policy-leases-and-offline-behavior",
  "endpoint-management": "local-ui",
  "gateway-aggregation": "architecture",
  identity: "admin-ui",
  contributing: "mac-install#build-your-own-archive",
  "dhivon-release": "checksums",
};
