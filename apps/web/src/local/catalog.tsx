import { useEffect, useState } from "react";
import { Search } from "lucide-react";
import type { LocalCatalog } from "@/generated/LocalCatalog";
import type { LocalMcpMetadata } from "@/generated/LocalMcpMetadata";
import type { SecretMetadata } from "@/generated/SecretMetadata";
import type { InstallProgress } from "@/generated/InstallProgress";
import type { LocalOAuthStatus } from "@/generated/LocalOAuthStatus";
import { Button } from "@/components/ui/button";
import { Spinner } from "@/components/ui/spinner";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Field,
  FieldGroup,
  FieldLabel,
  FieldDescription,
} from "@/components/ui/field";
import { Checkbox } from "@/components/ui/checkbox";
import {
  InputGroup,
  InputGroupInput,
  InputGroupAddon,
} from "@/components/ui/input-group";
import {
  Table,
  TableHeader,
  TableHead,
  TableBody,
  TableRow,
  TableCell,
} from "@/components/ui/table";
import { Alert, AlertTitle, AlertDescription } from "@/components/ui/alert";
import { Editor, Feedback, Choice, NoData } from "@/components/forms";
import type { LocalActions } from "./types";
import * as decode from "./contracts";

export function authName(auth: string) {
  return auth === "bearer"
    ? "Bearer token"
    : auth === "oauth"
      ? "OAuth"
      : "No authentication";
}
export function transportName(transport: string) {
  return transport === "stdio" ? "Local STDIO" : "Streamable HTTP";
}
function availabilityMessage(mcp: LocalMcpMetadata) {
  if (mcp.toolPolicy.mode === "allowlist" && mcp.toolPolicy.tools.length === 0)
    return "Tools blocked";
  if (!mcp.enabled) return null;
  return (
    (
      {
        missing_executable: "Missing executable",
        auth_required: "Authentication required",
        auth_rejected: "Credential rejected",
        token_expired: "Token expired",
        credential_sync_required: "Credential sync required",
        install_failed: "Install failed",
      } as Record<string, string>
    )[mcp.availability] ?? null
  );
}
export function LocalCatalogScreen({
  actions,
  valid,
}: {
  actions: LocalActions;
  valid: boolean;
}) {
  const [data, setData] = useState<LocalCatalog | null>(null),
    [error, setError] = useState<unknown>(null),
    [loading, setLoading] = useState(true),
    [search, setSearch] = useState(""),
    [selection, setSelection] = useState<{
      mcp: LocalMcpMetadata;
      revision: number;
      install: boolean;
    } | null>(null),
    [notice, setNotice] = useState("");
  const { read, generation } = actions;
  const revision = actions.context.status.configuration.revision;
  useEffect(() => {
    const abort = new AbortController();
    if (valid) {
      void read("/api/catalog", decode.catalog, abort.signal)
        .then((c) => {
          if (c.generation !== generation)
            throw new Error("Identity changed. Reload the page.");
          if (!abort.signal.aborted) {
            setData(c);
            setError(null);
            setLoading(false);
          }
        })
        .catch((e) => {
          if (!abort.signal.aborted) {
            setData(null);
            setError(e);
            setLoading(false);
          }
        });
    }
    return () => abort.abort();
  }, [valid, revision, read, generation]);
  const visible =
    data?.mcps.filter((m) =>
      `${m.name} ${m.description}`.toLowerCase().includes(search.toLowerCase()),
    ) ?? [];
  function saved(catalog: LocalCatalog) {
    if (catalog.generation !== generation) return;
    setData(catalog);
    setSelection(null);
    setNotice("Server confirmed the current settings.");
  }
  if (!valid)
    return (
      <NoData
        title="Refresh required"
        description="No tools are available until this Mac receives a complete valid configuration."
      />
    );
  return (
    <>
      <Feedback error={error} reload={actions.reload} />
      {notice ? (
        <p role="status" className="save-notice">
          {notice}
        </p>
      ) : null}
      <InputGroup className="local-search">
        <InputGroupAddon>
          <Search aria-hidden />
        </InputGroupAddon>
        <InputGroupInput
          aria-label="Search MCPs"
          placeholder="Search MCPs"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
      </InputGroup>
      {loading && !data ? (
        <Skeleton className="h-80 w-full" />
      ) : data && visible.length ? (
        <div className="data-table local-table">
          <Table>
            <TableHeader>
              <TableRow>
                {["MCP", "Connection", "State", "Actions"].map((t) => (
                  <TableHead key={t} scope="col">
                    {t}
                  </TableHead>
                ))}
              </TableRow>
            </TableHeader>
            <TableBody>
              {visible.map((m) => (
                <TableRow key={m.id}>
                  <TableCell>
                    <strong>{m.name}</strong>
                    <p className="muted">{transportName(m.transport)}</p>
                  </TableCell>
                  <TableCell>{authName(m.auth)}</TableCell>
                  <TableCell>
                    <span className="mcp-state">
                      <span
                        className={
                          m.enabled
                            ? "connection-dot"
                            : "connection-dot disabled"
                        }
                        aria-hidden
                      />
                      {m.enabled ? "Enabled" : "Disabled"}
                    </span>
                    {availabilityMessage(m) ? (
                      <p className="muted">{availabilityMessage(m)}</p>
                    ) : null}
                  </TableCell>
                  <TableCell>
                    <div className="local-row-actions">
                      {m.installationAvailable && !m.installed && !m.enabled ? (
                        <Button
                          variant="outline"
                          disabled={!actions.online}
                          onClick={() =>
                            setSelection({
                              mcp: m,
                              revision: data.revision,
                              install: true,
                            })
                          }
                        >
                          Install
                        </Button>
                      ) : null}
                      <Button
                        variant="outline"
                        onClick={() =>
                          setSelection({
                            mcp: m,
                            revision: data.revision,
                            install: false,
                          })
                        }
                      >
                        Manage
                      </Button>
                    </div>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      ) : !error ? (
        <NoData
          title={search ? "No matching MCPs" : "No MCPs assigned"}
          description={
            search
              ? "Try another name."
              : "Ask your administrator to assign an MCP to your account or group."
          }
        />
      ) : null}
      <p className="local-footer muted">
        Only MCPs assigned to you are shown. Your administrator controls tool
        access.
      </p>
      {selection ? (
        <Manage
          key={selection.mcp.id}
          actions={actions}
          {...selection}
          onSaved={saved}
          onClose={() => setSelection(null)}
        />
      ) : null}
    </>
  );
}
function Manage({
  actions,
  mcp,
  revision,
  install,
  onSaved,
  onClose,
}: {
  actions: LocalActions;
  mcp: LocalMcpMetadata;
  revision: number;
  install: boolean;
  onSaved: (value: LocalCatalog) => void;
  onClose: () => void;
}) {
  const [secretId, setSecretId] = useState(mcp.secret?.id ?? ""),
    [keys, setKeys] = useState<SecretMetadata[] | null>(null),
    [oauth, setOAuth] = useState<LocalOAuthStatus | null>(null),
    [flow, setFlow] = useState<{ id: string; url: string } | null>(null),
    [progress, setProgress] = useState<InstallProgress | null>(null),
    [enableAfter, setEnableAfter] = useState(false),
    [confirmInstall, setConfirmInstall] = useState(install),
    [rerun, setRerun] = useState(false),
    [busy, setBusy] = useState(false),
    [error, setError] = useState<unknown>(null),
    [readError, setReadError] = useState<unknown>(null),
    [notice, setNotice] = useState("");
  const { read, online, generation } = actions;
  useEffect(() => {
    const abort = new AbortController();
    if (online && mcp.auth === "bearer")
      void read("/api/secrets", decode.secrets, abort.signal)
        .then((v) => {
          if (!abort.signal.aborted) {
            setKeys(v);
            setReadError(null);
          }
        })
        .catch((e) => {
          if (!abort.signal.aborted) setReadError(e);
        });
    return () => abort.abort();
  }, [online, mcp.auth, read]);
  useEffect(() => {
    const abort = new AbortController();
    let running = false;
    async function load() {
      if (running) return;
      running = true;
      try {
        const value = await read(
          `/api/oauth/${decode.segment(mcp.id)}/status`,
          decode.oauth,
          abort.signal,
        );
        if (!abort.signal.aborted) {
          setOAuth(value);
          setReadError(null);
        }
      } catch (e) {
        if (!abort.signal.aborted) setReadError(e);
      } finally {
        running = false;
      }
    }
    if (online && mcp.auth === "oauth") void load();
    const timer = flow && online ? setInterval(() => void load(), 5000) : null;
    return () => {
      abort.abort();
      if (timer) clearInterval(timer);
    };
  }, [online, mcp.auth, mcp.id, read, flow]);
  useEffect(() => {
    if (!progress || progress.state !== "running") return;
    const abort = new AbortController();
    let running = false;
    const id = progress.id;
    async function poll() {
      if (running) return;
      running = true;
      try {
        const value = await read(
          `/api/installations/${decode.segment(id)}`,
          decode.progress,
          abort.signal,
        );
        if (value.generation !== generation)
          throw new Error("Identity changed");
        if (!abort.signal.aborted) {
          setProgress(value);
          setReadError(null);
        }
      } catch (e) {
        if (!abort.signal.aborted) setReadError(e);
      } finally {
        running = false;
      }
    }
    const timer = setInterval(() => void poll(), 1000);
    return () => {
      abort.abort();
      clearInterval(timer);
    };
  }, [progress, read, generation]);
  async function perform(operation: () => Promise<void>) {
    if (busy) return;
    setBusy(true);
    setError(null);
    setNotice("");
    try {
      await operation();
    } catch (e) {
      setError(e);
    } finally {
      setBusy(false);
    }
  }
  async function settings(enabled: boolean) {
    await perform(async () => {
      const value = await actions.write(
        `/api/settings/${decode.segment(mcp.id)}`,
        "PUT",
        {
          expectedRevision: revision,
          enabled,
          secretId: enabled && mcp.auth === "bearer" ? secretId : null,
        },
        decode.catalog,
      );
      onSaved(value);
    });
  }
  async function startInstall() {
    await perform(async () => {
      const value = await actions.write(
        `/api/mcps/${decode.segment(mcp.id)}/install`,
        "POST",
        {
          expectedRevision: revision,
          enableAfterInstall: enableAfter,
          secretId: enableAfter && mcp.auth === "bearer" ? secretId : null,
          rerun,
        },
        decode.progress,
      );
      if (value.generation !== generation) throw new Error("Identity changed");
      setProgress(value);
      setConfirmInstall(false);
    });
  }
  function connect() {
    // Create a blank browsing context synchronously with the human action; provider navigation follows native validation.
    const popup = window.open("about:blank", "_blank");
    if (popup) popup.opener = null;
    void perform(async () => {
      try {
        const result = await actions.write(
          `/api/oauth/${decode.segment(mcp.id)}/connect`,
          "POST",
          { expectedRevision: revision },
          decode.consent,
        );
        const url = new URL(result.authorizationUrl);
        const fixture =
          actions.context.serverUrl.startsWith("http://127.0.0.1:");
        if (
          url.username ||
          url.password ||
          (url.protocol !== "https:" &&
            !(
              fixture &&
              url.protocol === "http:" &&
              url.hostname === "127.0.0.1"
            ))
        )
          throw new Error(
            "The provider returned an unsupported authorization URL.",
          );
        setFlow({ id: result.flowId, url: url.href });
        if (popup) popup.location.href = url.href;
        setNotice(
          "Complete authorization in the provider window, then reload current state before enabling.",
        );
      } catch (e) {
        popup?.close();
        throw e;
      }
    });
  }
  const locked = !online || busy || Boolean(error),
    installing = progress?.state === "running";
  const readiness = oauth
    ? ((
        {
          ready: "Ready on this Mac",
          auth_rejected: "Credential rejected — reconnect",
          token_expired: "Token expired — reconnect",
          credential_sync_required: "Credential sync required",
          disconnected: "Connect OAuth to continue",
          authorization_pending: "Authorization is pending",
        } as Record<string, string>
      )[oauth.readiness] ?? "Authentication unavailable")
    : online
      ? "Checking OAuth status"
      : "Reconnect to check OAuth status";
  function close() {
    if (installing) {
      setNotice("Cancel the installation before closing this view.");
      return;
    }
    if (flow) {
      setNotice(
        "Cancel the authorization or reload current state before closing this view.",
      );
      return;
    }
    onClose();
  }
  return (
    <Editor
      title={mcp.name}
      description={`${authName(mcp.auth)} · ${transportName(mcp.transport)}`}
      onClose={close}
      busy={busy}
    >
      <div className="local-editor editor-form">
        <div className="editor-body">
          <p className="muted">
            {mcp.enabled ? "Enabled on the server" : "Disabled"}
          </p>
          {mcp.description ? (
            <p className="local-description">{mcp.description}</p>
          ) : null}
          <Feedback error={error ?? readError} reload={actions.reload} />
          {mcp.availability === "missing_executable" ? (
            <Alert>
              <AlertTitle>Missing executable</AlertTitle>
              <AlertDescription>
                Install the required program or ask your administrator to
                correct its executable or working directory.
              </AlertDescription>
            </Alert>
          ) : null}
          {notice ? (
            <Alert>
              <AlertDescription>{notice}</AlertDescription>
            </Alert>
          ) : null}
          {!online ? (
            <Alert>
              <AlertTitle>Offline · Read-only</AlertTitle>
              <AlertDescription>
                Reconnect before changing settings or authentication.
              </AlertDescription>
            </Alert>
          ) : null}
          <section className="local-section">
            <h2>Connection</h2>
            <FieldGroup>
              {mcp.auth === "bearer" ? (
                <>
                  <Choice
                    label="Secret key"
                    value={secretId || "choose"}
                    onChange={(v) => setSecretId(v === "choose" ? "" : v)}
                    disabled={locked || keys === null}
                    choices={[
                      { value: "choose", label: "Choose a key" },
                      ...(keys ?? []).map((s) => ({
                        value: s.id,
                        label: s.key,
                      })),
                    ]}
                  />
                  <FieldDescription>
                    Choose a key from your encrypted secret store. Values are
                    never shown here.
                  </FieldDescription>
                  <Button
                    variant="link"
                    type="button"
                    onClick={() => {
                      if (!busy && !installing && !flow) {
                        onClose();
                        location.hash = "secrets";
                      }
                    }}
                  >
                    Manage secrets
                  </Button>
                </>
              ) : mcp.auth === "oauth" ? (
                <>
                  <p>{readiness}</p>
                  {oauth?.connected ? (
                    <p className="muted">
                      Connected on the server. Access token expires{" "}
                      {oauth.expiresAt
                        ? new Date(oauth.expiresAt * 1000).toLocaleString()
                        : "at an unknown time"}
                      .
                    </p>
                  ) : null}
                  <div className="local-row-actions">
                    <Button
                      variant="outline"
                      disabled={locked || Boolean(flow)}
                      onClick={connect}
                    >
                      {oauth?.connected ? "Reconnect OAuth" : "Connect OAuth"}
                    </Button>
                    <Button
                      variant="outline"
                      disabled={locked || oauth === null || Boolean(flow)}
                      onClick={() =>
                        void perform(async () => {
                          await actions.write(
                            `/api/oauth/${decode.segment(mcp.id)}/disconnect`,
                            "POST",
                            {
                              expectedRevision: revision,
                              expectedVersion: oauth?.version ?? 0,
                            },
                            decode.confirmed,
                          );
                          setFlow(null);
                          actions.reload();
                        })
                      }
                    >
                      Disconnect
                    </Button>
                  </div>
                  {flow ? (
                    <>
                      <p className="muted">
                        {oauth?.connected
                          ? "Authorization completed. Reload current state before enabling."
                          : "Authorization is pending. It expires after five minutes."}
                      </p>
                      <div className="local-row-actions">
                        <Button
                          variant="outline"
                          onClick={() =>
                            window.open(
                              flow.url,
                              "_blank",
                              "noopener,noreferrer",
                            )
                          }
                        >
                          Open authorization
                        </Button>
                        <Button
                          variant="outline"
                          disabled={busy}
                          onClick={() =>
                            void perform(async () => {
                              await actions.write(
                                `/api/oauth/flows/${decode.segment(flow.id)}/cancel`,
                                "POST",
                                {},
                                decode.acknowledgement,
                              );
                              setFlow(null);
                              setNotice(
                                "Authorization cancelled. Reload current state if a response was already issued.",
                              );
                            })
                          }
                        >
                          Cancel authorization
                        </Button>
                      </div>
                      {oauth?.connected ? (
                        <Button variant="outline" onClick={actions.reload}>
                          Reload current state
                        </Button>
                      ) : null}
                    </>
                  ) : null}
                </>
              ) : (
                <p>No authentication is required.</p>
              )}
            </FieldGroup>
          </section>
          {mcp.installationAvailable ? (
            <section className="local-section">
              <h2>Installation</h2>
              <p className="muted">
                {mcp.installed
                  ? "This command was installed on this Mac. Enablement is saved separately on the server."
                  : "Your administrator defined an installation command. It runs with your permissions on this Mac."}
              </p>
              {progress ? (
                <Alert role="status">
                  <AlertTitle>
                    {progress.state === "running"
                      ? "Installing"
                      : progress.state === "installed_disabled"
                        ? "Installed on this Mac"
                        : progress.state === "enabled"
                          ? "Installation and enablement confirmed"
                          : progress.state === "failed"
                            ? "Install failed"
                            : `Installation ${progress.state}`}
                  </AlertTitle>
                  <AlertDescription>
                    {progress.state === "running"
                      ? `${progress.elapsedSeconds}s elapsed. Output is kept private.`
                      : "Reload current configuration to review server enablement before another action."}
                  </AlertDescription>
                </Alert>
              ) : null}
              {installing ? (
                <Button
                  variant="outline"
                  disabled={busy}
                  onClick={() =>
                    void perform(async () => {
                      await actions.write(
                        `/api/installations/${decode.segment(progress.id)}/cancel`,
                        "POST",
                        null,
                        decode.acknowledgement,
                      );
                      setNotice(
                        "Cancellation requested. Waiting for owned work to stop.",
                      );
                    })
                  }
                >
                  Cancel installation
                </Button>
              ) : confirmInstall ? (
                <FieldGroup>
                  {mcp.installed ? (
                    <Alert>
                      <AlertTitle>Run installation again?</AlertTitle>
                      <AlertDescription>
                        This reruns the administrator's command and may change
                        software on your Mac.
                      </AlertDescription>
                    </Alert>
                  ) : null}
                  <Field orientation="horizontal">
                    <Checkbox
                      id="enable-after"
                      checked={enableAfter}
                      onCheckedChange={(v) => setEnableAfter(v === true)}
                      disabled={locked}
                    />
                    <FieldLabel htmlFor="enable-after">
                      Enable after successful installation
                    </FieldLabel>
                  </Field>
                  <Button
                    disabled={
                      locked ||
                      mcp.enabled ||
                      (enableAfter && mcp.auth === "bearer" && !secretId)
                    }
                    onClick={() => void startInstall()}
                  >
                    {mcp.installed ? "Confirm reinstall" : "Run installation"}
                  </Button>
                  <Button
                    variant="outline"
                    onClick={() => setConfirmInstall(false)}
                  >
                    Cancel
                  </Button>
                </FieldGroup>
              ) : (
                <div className="local-row-actions">
                  <Button
                    variant="outline"
                    disabled={locked || mcp.enabled}
                    onClick={() => {
                      setRerun(mcp.installed);
                      setConfirmInstall(true);
                    }}
                  >
                    {mcp.installed ? "Reinstall" : "Install"}
                  </Button>
                  {progress && progress.state !== "running" ? (
                    <Button variant="outline" onClick={actions.reload}>
                      Reload current state
                    </Button>
                  ) : null}
                </div>
              )}
            </section>
          ) : null}
          <section className="local-section">
            <h2>Tool access</h2>
            <p className="muted">
              {mcp.toolPolicy.mode === "allowlist"
                ? `Your administrator allows ${mcp.toolPolicy.tools.length} ${mcp.toolPolicy.tools.length === 1 ? "tool" : "tools"}.`
                : `Your administrator blocks ${mcp.toolPolicy.tools.length} named tools. Other discovered tools are permitted.`}
            </p>
            {mcp.toolPolicy.mode === "allowlist" &&
            !mcp.toolPolicy.tools.length ? (
              <Alert>
                <AlertTitle>Tools blocked by administrator</AlertTitle>
                <AlertDescription>
                  An empty allowlist exposes no tools, even when this MCP is
                  enabled.
                </AlertDescription>
              </Alert>
            ) : null}
            {mcp.toolPolicy.tools.length ? (
              <ul className="local-tool-list">
                {mcp.toolPolicy.tools.map((t) => (
                  <li key={t}>{t}</li>
                ))}
              </ul>
            ) : null}
          </section>
        </div>
        <footer className="editor-actions local-editor-actions">
          <Button
            disabled={
              locked ||
              Boolean(installing) ||
              Boolean(flow) ||
              (!mcp.enabled && mcp.auth === "bearer" && !secretId) ||
              (!mcp.enabled &&
                mcp.auth === "oauth" &&
                oauth?.readiness !== "ready")
            }
            onClick={() => void settings(!mcp.enabled)}
          >
            {busy ? <Spinner data-icon="inline-start" /> : null}
            {mcp.enabled ? "Disable MCP" : "Enable MCP"}
          </Button>
          <p className="muted">
            Enablement is saved on the server. Changes require a connection.
          </p>
        </footer>
      </div>
    </Editor>
  );
}
