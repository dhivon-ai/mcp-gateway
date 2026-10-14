import { useEffect, useState, type ReactNode } from "react";
import type { SecretMetadata } from "@/generated/SecretMetadata";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Field,
  FieldGroup,
  FieldLabel,
  FieldDescription,
} from "@/components/ui/field";
import {
  Table,
  TableHeader,
  TableHead,
  TableBody,
  TableRow,
  TableCell,
} from "@/components/ui/table";
import { Spinner } from "@/components/ui/spinner";
import { Skeleton } from "@/components/ui/skeleton";
import { Alert, AlertDescription } from "@/components/ui/alert";
import { Editor, Feedback, TextField, NoData } from "@/components/forms";
import type { LocalActions } from "./types";
import * as decode from "./contracts";
type Selection = {
  kind: "create" | "rename" | "replace" | "delete";
  secret?: SecretMetadata;
};
export function Secrets({
  actions,
  freshness,
}: {
  actions: LocalActions;
  freshness: ReactNode;
}) {
  const [keys, setKeys] = useState<SecretMetadata[] | null>(null),
    [selection, setSelection] = useState<Selection | null>(null),
    [error, setError] = useState<unknown>(null),
    [notice, setNotice] = useState(""),
    [request, setRequest] = useState(0);
  const { read, online } = actions;
  useEffect(() => {
    const abort = new AbortController();
    if (online)
      void read("/api/secrets", decode.secrets, abort.signal)
        .then((v) => {
          if (!abort.signal.aborted) {
            setKeys(v);
            setError(null);
          }
        })
        .catch((e) => {
          if (!abort.signal.aborted) {
            setKeys(null);
            setError(e);
          }
        });
    return () => abort.abort();
  }, [online, read, request]);
  function saved(synchronized: boolean) {
    setSelection(null);
    setKeys(null);
    setNotice(
      synchronized
        ? "Secret change confirmed. Values remain write-only."
        : "Secret change confirmed, but configuration sync failed. Refresh before changing MCP settings.",
    );
    setRequest((n) => n + 1);
  }
  return (
    <>
      <header className="page-header">
        <div>
          <h1>Your secrets</h1>
          <p>Encrypted on the server. Values are write-only.</p>
        </div>
        <Button
          disabled={!online || Boolean(error)}
          onClick={() => setSelection({ kind: "create" })}
        >
          Add secret
        </Button>
      </header>
      {freshness}
      <Feedback error={error} reload={actions.reload} />
      {notice ? (
        <Alert role="status">
          <AlertDescription>{notice}</AlertDescription>
        </Alert>
      ) : null}
      {!online ? (
        <NoData
          title="Secrets require a connection"
          description="Reconnect to view key metadata or change your encrypted secrets."
        />
      ) : keys === null && !error ? (
        <Skeleton className="h-48 w-full" />
      ) : keys?.length ? (
        <div className="data-table local-table secrets-table">
          <Table>
            <TableHeader>
              <TableRow>
                {["Key", "Version", "Actions"].map((t) => (
                  <TableHead key={t} scope="col">
                    {t}
                  </TableHead>
                ))}
              </TableRow>
            </TableHeader>
            <TableBody>
              {keys.map((s) => (
                <TableRow key={s.id}>
                  <TableCell>{s.key}</TableCell>
                  <TableCell>{s.version}</TableCell>
                  <TableCell>
                    <div className="local-row-actions">
                      <Button
                        variant="outline"
                        onClick={() =>
                          setSelection({ kind: "rename", secret: s })
                        }
                      >
                        Rename
                      </Button>
                      <Button
                        variant="outline"
                        onClick={() =>
                          setSelection({ kind: "replace", secret: s })
                        }
                      >
                        Replace value
                      </Button>
                      <Button
                        variant="outline"
                        onClick={() =>
                          setSelection({ kind: "delete", secret: s })
                        }
                      >
                        Delete
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
          title="No secret keys yet"
          description="Add a token, then choose its key when enabling a bearer MCP."
        />
      ) : null}
      <p className="local-footer muted">
        Secret values are never returned to this browser.
      </p>
      {selection ? (
        <SecretEditor
          actions={actions}
          selection={selection}
          onClose={() => setSelection(null)}
          onSaved={saved}
        />
      ) : null}
    </>
  );
}
function SecretEditor({
  actions,
  selection,
  onClose,
  onSaved,
}: {
  actions: LocalActions;
  selection: Selection;
  onClose: () => void;
  onSaved: (synchronized: boolean) => void;
}) {
  const { kind, secret } = selection;
  const [key, setKey] = useState(secret?.key ?? ""),
    [value, setValue] = useState(""),
    [busy, setBusy] = useState(false),
    [error, setError] = useState<unknown>(null);
  const title =
    kind === "create"
      ? "Add secret"
      : kind === "rename"
        ? "Rename secret"
        : kind === "replace"
          ? "Replace value"
          : "Delete secret";
  const description =
    kind === "create"
      ? "Store a token for your remote MCPs."
      : kind === "delete"
        ? "Remove this key from your encrypted secret store."
        : `Update ${secret?.key ?? "this key"}.`;
  async function submit() {
    if (busy || !actions.online || error) return;
    setBusy(true);
    setError(null);
    try {
      const path = secret
        ? `/api/secrets/${decode.segment(secret.id)}`
        : "/api/secrets";
      const result = await actions.write(
        kind === "rename" ? `${path}/key` : path,
        kind === "delete" ? "DELETE" : kind === "create" ? "POST" : "PUT",
        kind === "delete"
          ? { expectedVersion: secret?.version }
          : kind === "rename"
            ? { key, expectedVersion: secret?.version }
            : { key, value, expectedVersion: secret?.version ?? null },
        decode.outcome,
      );
      if (!result.confirmed)
        throw new Error(
          "The server did not confirm this change. Reload current state.",
        );
      onSaved(result.synchronized);
    } catch (e) {
      setError(e);
    } finally {
      setValue("");
      setBusy(false);
    }
  }
  return (
    <Editor
      title={title}
      description={description}
      onClose={onClose}
      busy={busy}
    >
      <form
        className="local-editor secret-editor editor-form"
        onSubmit={(e) => {
          e.preventDefault();
          void submit();
        }}
      >
        <div className="editor-body">
          <Feedback error={error} reload={actions.reload} />
          <FieldGroup>
            {kind === "delete" ? (
              <Alert>
                <AlertDescription>
                  Delete key <strong>{secret?.key}</strong>? Disable or remove
                  active MCP bindings before deleting. This cannot be undone.
                </AlertDescription>
              </Alert>
            ) : (
              <TextField
                label="Key"
                value={key}
                onChange={setKey}
                maxLength={128}
                required
                disabled={busy || kind === "replace" || !actions.online}
              />
            )}{" "}
            {kind === "create" || kind === "replace" ? (
              <Field data-disabled={busy || !actions.online}>
                <FieldLabel htmlFor="secret-value">Value</FieldLabel>
                <Input
                  id="secret-value"
                  type="password"
                  autoComplete="off"
                  spellCheck={false}
                  required
                  minLength={1}
                  maxLength={16384}
                  disabled={busy || !actions.online}
                  value={value}
                  onChange={(e) => setValue(e.target.value)}
                  aria-describedby="secret-value-hint"
                />
                <FieldDescription id="secret-value-hint">
                  Values cannot be viewed after saving.
                </FieldDescription>
              </Field>
            ) : null}
          </FieldGroup>
        </div>
        <footer className="editor-actions local-editor-actions">
          <Button
            variant={kind === "delete" ? "destructive" : "default"}
            disabled={busy || !actions.online || Boolean(error)}
          >
            {busy ? <Spinner data-icon="inline-start" /> : null}
            {kind === "delete"
              ? "Delete secret"
              : kind === "rename"
                ? "Save key"
                : "Save secret"}
          </Button>
        </footer>
      </form>
    </Editor>
  );
}
