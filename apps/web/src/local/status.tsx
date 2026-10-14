import { useState } from "react";
import { Copy } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { LocalContext } from "./contracts";
function date(value: number | null) {
  return value === null
    ? "Unavailable"
    : new Date(value * 1000).toLocaleString();
}
export function Status({
  context,
  valid,
}: {
  context: LocalContext;
  valid: boolean;
}) {
  const [copy, setCopy] = useState("");
  const config = context.status.configuration;
  return (
    <section className="local-status">
      <dl>
        {[
          ["Server", context.serverUrl],
          [
            "Configuration",
            valid
              ? context.status.online
                ? "Valid configuration"
                : "Valid cached configuration"
              : "Refresh required",
          ],
          ["Revision", config.revision ?? "Unavailable"],
          ["Last validated", date(config.validatedAt)],
          ["Expires", date(config.expiresAt)],
        ].map(([label, value]) => (
          <div key={label}>
            <dt>{label}</dt>
            <dd>{value}</dd>
          </div>
        ))}
      </dl>
      <section className="agent-setup">
        <h2>Coding agent setup</h2>
        <p className="muted">
          Configure one STDIO entry for your coding agent.
        </p>
        <div className="agent-command">
          <code>mcp-gateway mcp</code>
          <Button
            variant="ghost"
            size="icon"
            aria-label="Copy STDIO command"
            onClick={() => {
              void navigator.clipboard.writeText("mcp-gateway mcp").then(
                () => setCopy("Copied"),
                () => setCopy("Copy unavailable. Select the command manually."),
              );
            }}
          >
            <Copy />
          </Button>
        </div>
        {copy ? <p role="status">{copy}</p> : null}
        <p className="muted">
          At expiry, the gateway stops exposing tools until a complete refresh
          succeeds.
        </p>
      </section>
    </section>
  );
}
