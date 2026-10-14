import { useState } from "react";
import { Plus, Search, Trash2 } from "lucide-react";
import type { McpDefinition } from "@/generated/McpDefinition";
import type { CommandSpec } from "@/generated/CommandSpec";
import type { AuthMode } from "@/generated/AuthMode";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Field,
  FieldGroup,
  FieldLabel,
  FieldSet,
  FieldLegend,
  FieldDescription,
} from "@/components/ui/field";
import {
  InputGroup,
  InputGroupInput,
  InputGroupAddon,
  InputGroupButton,
} from "@/components/ui/input-group";
import { RadioGroup, RadioGroupItem } from "@/components/ui/radio-group";
import {
  Table,
  TableHeader,
  TableBody,
  TableRow,
  TableHead,
  TableCell,
} from "@/components/ui/table";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import {
  Editor,
  Feedback,
  TextField,
  Choice,
  NoData,
} from "@/components/forms";
import { accessLabel, AssignmentRows, AssignmentEditor } from "./access";
import { ConfirmDelete } from "./identities";
import type { AdminData, Actions } from "./types";
import { cn } from "@/lib/utils";
import * as decode from "@/lib/decode";

interface CommandDraft {
  program: string;
  args: string;
  env: string;
  cwd: string;
}
function draft(command?: CommandSpec | null): CommandDraft {
  return {
    program: command?.program ?? "",
    args: JSON.stringify(command?.args ?? [], null, 2),
    env: JSON.stringify(command?.env ?? {}, null, 2),
    cwd: command?.cwd ?? "",
  };
}
function parseCommand(value: CommandDraft): CommandSpec {
  try {
    return decode.command({
      program: value.program,
      args: JSON.parse(value.args) as unknown,
      env: JSON.parse(value.env) as unknown,
      cwd: value.cwd || null,
    });
  } catch {
    throw new Error(
      "Arguments must be a bounded JSON array of strings; environment must be a JSON object with string values.",
    );
  }
}
function CommandFields({
  value,
  onChange,
  prefix,
}: {
  value: CommandDraft;
  onChange: (value: CommandDraft) => void;
  prefix: string;
}) {
  return (
    <FieldSet>
      <FieldLegend>{prefix}</FieldLegend>
      <FieldGroup>
        <TextField
          label={`${prefix} executable`}
          value={value.program}
          onChange={(program) => onChange({ ...value, program })}
          required
        />
        <TextField
          label={`${prefix} arguments`}
          multiline
          value={value.args}
          onChange={(args) => onChange({ ...value, args })}
          maxLength={2097152}
          description={
            'JSON array of strings, for example ["--port", "8080"]. Use [] for no arguments and "\\n" for a literal newline. No shell expansion.'
          }
        />
        <TextField
          label={`${prefix} environment`}
          multiline
          value={value.env}
          onChange={(env) => onChange({ ...value, env })}
          maxLength={2097152}
          description={
            'JSON object of string values, for example {"MODE": "work"}. Use {} for no environment values. Use the user secret store for bearer tokens.'
          }
        />
        <TextField
          label="Working directory"
          value={value.cwd}
          onChange={(cwd) => onChange({ ...value, cwd })}
        />
      </FieldGroup>
    </FieldSet>
  );
}
export function policyLabel(mcp: McpDefinition): string {
  const p = mcp.toolPolicy;
  return p.mode === "allowlist"
    ? p.tools.length
      ? `${p.tools.length} allowed`
      : "No tools allowed"
    : p.tools.length
      ? `${p.tools.length} blocked`
      : "All tools";
}
function CatalogEditor({
  initial,
  data,
  actions,
  onClose,
}: {
  initial: McpDefinition | null;
  data: AdminData;
  actions: Actions;
  onClose: () => void;
}) {
  const [id] = useState(initial?.id ?? crypto.randomUUID());
  const [revision] = useState(data.config.revision);
  const [tab, setTab] = useState("connection");
  const [name, setName] = useState(initial?.name ?? "");
  const [description, setDescription] = useState(initial?.description ?? "");
  const [active, setActive] = useState(initial?.active ?? true);
  const [kind, setKind] = useState(initial?.transport.kind ?? "stdio");
  const [runtime, setRuntime] = useState(
    draft(
      initial?.transport.kind === "stdio" ? initial.transport.command : null,
    ),
  );
  const [url, setUrl] = useState(
    initial?.transport.kind === "http" ? initial.transport.url : "",
  );
  const [auth, setAuth] = useState<AuthMode["kind"]>(
    initial?.auth.kind ?? "none",
  );
  const [clientId, setClientId] = useState(
    initial?.auth.kind === "oauth" ? initial.auth.clientId : "",
  );
  const [scopes, setScopes] = useState(
    initial?.auth.kind === "oauth" ? initial.auth.scopes.join("\n") : "",
  );
  const [installEnabled, setInstallEnabled] = useState(
    Boolean(initial?.install),
  );
  const [install, setInstall] = useState(draft(initial?.install));
  const [policy, setPolicy] = useState(
    initial?.toolPolicy ?? { mode: "allowlist" as const, tools: [] },
  );
  const [manual, setManual] = useState("");
  const [validation, setValidation] = useState<Error | null>(null);
  const [deleting, setDeleting] = useState(false);
  const [assigning, setAssigning] = useState(false);
  const observed =
    data.config.inventory.find((i) => i.mcpId === id)?.names ?? [];
  const names = [...new Set([...observed, ...policy.tools])];
  async function submit() {
    try {
      setValidation(null);
      const definition: McpDefinition = {
        id,
        name,
        description,
        active,
        transport:
          kind === "stdio"
            ? { kind: "stdio", command: parseCommand(runtime) }
            : { kind: "http", url },
        auth:
          kind === "stdio" || auth === "none"
            ? { kind: "none" }
            : auth === "bearer"
              ? { kind: "bearer" }
              : {
                  kind: "oauth",
                  clientId,
                  scopes: scopes ? scopes.split("\n") : [],
                },
        install: installEnabled ? parseCommand(install) : null,
        toolPolicy: policy,
      };
      if (
        await actions.execute(
          initial ? `/api/admin/mcps/${id}` : "/api/admin/mcps",
          initial ? "PUT" : "POST",
          { expectedRevision: revision, definition },
        )
      )
        onClose();
    } catch (e) {
      setValidation(
        e instanceof Error ? e : new Error("Check the connection fields."),
      );
    }
  }
  function addTool() {
    const value = manual.trim();
    if (
      !value ||
      value.length > 512 ||
      Array.from(value).some((c) => c.charCodeAt(0) < 32)
    ) {
      setValidation(
        new Error("Enter an original tool name with 1–512 characters."),
      );
      return;
    }
    setPolicy((old) => ({
      ...old,
      tools: [...new Set([...old.tools, value])],
    }));
    setManual("");
    setValidation(null);
  }
  return (
    <Editor
      title={initial?.name ?? "Add MCP"}
      description="Configure connection, assignments and original tool names."
      onClose={onClose}
      wide
      hideDescription
      busy={actions.busy}
    >
      <form
        className="editor-form"
        onSubmit={(e) => {
          e.preventDefault();
          void submit();
        }}
      >
        <Tabs value={tab} onValueChange={setTab} className="editor-tabs">
          <TabsList variant="line">
            <TabsTrigger value="connection">Connection</TabsTrigger>
            <TabsTrigger value="access" disabled={!initial}>
              Access
            </TabsTrigger>
            <TabsTrigger value="tools">Tools</TabsTrigger>
          </TabsList>
          <div className="editor-body">
            <Feedback
              error={validation ?? actions.error}
              reload={actions.error ? actions.reload : undefined}
            />
            <TabsContent value="connection">
              <FieldGroup>
                <TextField
                  label="Name"
                  value={name}
                  onChange={setName}
                  required
                  maxLength={decode.MCP_NAME_BYTE_LIMIT}
                />
                <TextField
                  label="Description"
                  value={description}
                  onChange={setDescription}
                  multiline
                />
                <Field orientation="horizontal">
                  <Checkbox
                    id="mcp-active"
                    checked={active}
                    onCheckedChange={(v) => setActive(v === true)}
                  />
                  <FieldLabel htmlFor="mcp-active">Active</FieldLabel>
                </Field>
                <Choice
                  label="Transport"
                  value={kind}
                  onChange={(v) => {
                    if (v === "stdio" || v === "http") {
                      setKind(v);
                      if (v === "stdio") setAuth("none");
                    }
                  }}
                  choices={[
                    { value: "stdio", label: "STDIO" },
                    { value: "http", label: "Streamable HTTP" },
                  ]}
                />
                {kind === "stdio" ? (
                  <CommandFields
                    prefix="Runtime"
                    value={runtime}
                    onChange={setRuntime}
                  />
                ) : (
                  <>
                    <TextField
                      label="Endpoint URL"
                      type="url"
                      value={url}
                      onChange={setUrl}
                      required
                    />
                    <Choice
                      label="Authentication"
                      value={auth}
                      onChange={(v) => {
                        if (v === "none" || v === "bearer" || v === "oauth")
                          setAuth(v);
                      }}
                      choices={[
                        { value: "none", label: "No authentication" },
                        { value: "bearer", label: "Bearer token" },
                        { value: "oauth", label: "OAuth" },
                      ]}
                    />
                    {auth === "bearer" ? (
                      <FieldDescription>
                        Users select an owned key from their secret store when
                        enabling this MCP.
                      </FieldDescription>
                    ) : null}
                    {auth === "oauth" ? (
                      <>
                        <TextField
                          label="Public client ID"
                          value={clientId}
                          onChange={setClientId}
                          required
                          maxLength={512}
                        />
                        <TextField
                          label="Scopes"
                          value={scopes}
                          onChange={setScopes}
                          multiline
                          description="One scope per line. Register the native public client with the provider first."
                        />
                      </>
                    ) : null}
                  </>
                )}
                <Field orientation="horizontal">
                  <Checkbox
                    id="install-enabled"
                    checked={installEnabled}
                    onCheckedChange={(v) => setInstallEnabled(v === true)}
                  />
                  <FieldLabel htmlFor="install-enabled">
                    Installation command
                  </FieldLabel>
                </Field>
                {installEnabled ? (
                  <CommandFields
                    prefix="Installation"
                    value={install}
                    onChange={setInstall}
                  />
                ) : null}
              </FieldGroup>
            </TabsContent>
            <TabsContent value="access">
              <FieldGroup>
                <FieldDescription>
                  Grants add access. Removing the last grant clears enablement.
                </FieldDescription>
                <AssignmentRows
                  data={data}
                  actions={actions}
                  assignments={data.config.assignments.filter(
                    (a) => a.mcpId === id,
                  )}
                />
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => setAssigning(true)}
                  disabled={actions.busy}
                >
                  Add assignment
                </Button>
              </FieldGroup>
            </TabsContent>
            <TabsContent value="tools">
              <FieldGroup>
                <FieldSet>
                  <FieldLegend id="tool-policy-label">Tool policy</FieldLegend>
                  <RadioGroup
                    aria-labelledby="tool-policy-label"
                    value={policy.mode}
                    onValueChange={(mode) => {
                      if (mode === "allowlist" || mode === "denylist")
                        setPolicy((old) => ({ ...old, mode }));
                    }}
                  >
                    <Field orientation="horizontal">
                      <RadioGroupItem value="allowlist" id="allow-tools" />
                      <FieldLabel htmlFor="allow-tools">
                        Allow selected tools
                      </FieldLabel>
                    </Field>
                    <Field orientation="horizontal">
                      <RadioGroupItem value="denylist" id="deny-tools" />
                      <FieldLabel htmlFor="deny-tools">
                        Block selected tools
                      </FieldLabel>
                    </Field>
                  </RadioGroup>
                  <FieldDescription>
                    {policy.mode === "allowlist"
                      ? "New tools stay hidden until selected."
                      : "New unlisted tools are permitted for enabled, assigned MCPs."}
                  </FieldDescription>
                </FieldSet>
                <FieldSet>
                  <FieldLegend>Observed tools</FieldLegend>
                  {names.length ? (
                    <FieldGroup>
                      {names.map((n, i) => (
                        <Field orientation="horizontal" key={n}>
                          <Checkbox
                            id={`tool-${i}`}
                            checked={policy.tools.includes(n)}
                            onCheckedChange={(v) =>
                              setPolicy((old) => ({
                                ...old,
                                tools:
                                  v === true
                                    ? [...new Set([...old.tools, n])]
                                    : old.tools.filter((t) => t !== n),
                              }))
                            }
                          />
                          <FieldLabel
                            htmlFor={`tool-${i}`}
                            className="tool-name"
                          >
                            {n}
                            {!observed.includes(n) ? (
                              <span className="sr-only">
                                {" "}
                                (manually entered)
                              </span>
                            ) : null}
                          </FieldLabel>
                        </Field>
                      ))}
                    </FieldGroup>
                  ) : (
                    <NoData title="No observed tools" />
                  )}
                </FieldSet>
                <Field>
                  <FieldLabel htmlFor="manual-tool">Add tool name</FieldLabel>
                  <InputGroup>
                    <InputGroupInput
                      id="manual-tool"
                      placeholder="Original upstream name"
                      value={manual}
                      onChange={(e) => setManual(e.target.value)}
                      maxLength={512}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") {
                          e.preventDefault();
                          addTool();
                        }
                      }}
                    />
                    <InputGroupAddon align="inline-end">
                      <InputGroupButton
                        variant="outline"
                        size="sm"
                        onClick={addTool}
                      >
                        Add
                      </InputGroupButton>
                    </InputGroupAddon>
                  </InputGroup>
                  <FieldDescription>
                    Enter names manually before discovery.
                  </FieldDescription>
                </Field>
              </FieldGroup>
            </TabsContent>
          </div>
        </Tabs>
        <div className="editor-actions">
          {initial && tab === "connection" ? (
            <Button
              type="button"
              variant="destructive"
              disabled={actions.busy}
              onClick={() => setDeleting(true)}
            >
              <Trash2 data-icon="inline-start" />
              Delete MCP
            </Button>
          ) : null}
          <Button
            type="button"
            variant="outline"
            onClick={onClose}
            disabled={actions.busy}
          >
            Cancel
          </Button>
          <Button disabled={actions.busy}>
            {tab === "tools" ? "Save policy" : "Save MCP"}
          </Button>
        </div>
      </form>
      {deleting ? (
        <ConfirmDelete
          title={`Delete ${name}?`}
          description="Assignments, enablement and credential bindings will be removed."
          busy={actions.busy}
          onClose={() => setDeleting(false)}
          onConfirm={() => {
            void actions
              .execute(`/api/admin/mcps/${id}`, "DELETE", {
                expectedRevision: revision,
              })
              .then((ok) => {
                if (ok) onClose();
                else setDeleting(false);
              });
          }}
        />
      ) : null}
      {assigning ? (
        <AssignmentEditor
          data={data}
          actions={actions}
          mcpId={id}
          onClose={() => setAssigning(false)}
        />
      ) : null}
    </Editor>
  );
}
export function Catalog({
  data,
  actions,
}: {
  data: AdminData;
  actions: Actions;
}) {
  const [search, setSearch] = useState("");
  const [editing, setEditing] = useState<McpDefinition | null | undefined>();
  const rows = data.config.mcps.filter((m) =>
    `${m.name} ${m.description}`.toLowerCase().includes(search.toLowerCase()),
  );
  return (
    <>
      <header className="page-header">
        <div>
          <h1>MCP catalog</h1>
          <p>No access until assigned. Users choose what to enable.</p>
        </div>
        <Button onClick={() => setEditing(null)} disabled={actions.busy}>
          <Plus data-icon="inline-start" />
          Add MCP
        </Button>
      </header>
      <Field className="catalog-search">
        <FieldLabel htmlFor="catalog-search" className="sr-only">
          Search MCPs
        </FieldLabel>
        <InputGroup>
          <InputGroupAddon>
            <Search />
          </InputGroupAddon>
          <InputGroupInput
            id="catalog-search"
            placeholder="Search MCPs"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </InputGroup>
      </Field>
      {rows.length ? (
        <div className="data-table catalog-table">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead scope="col">MCP</TableHead>
                <TableHead scope="col">Transport</TableHead>
                <TableHead scope="col">Tool policy</TableHead>
                <TableHead scope="col">Access</TableHead>
                <TableHead scope="col">
                  <span className="sr-only">Actions</span>
                </TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {rows.map((m) => (
                <TableRow key={m.id}>
                  <TableCell>
                    <div className="mcp-name">
                      <span
                        className={cn(
                          "eligibility-dot",
                          m.active && accessLabel(m.id, data) !== "Unassigned"
                            ? "eligible"
                            : "unassigned",
                        )}
                        role="img"
                        aria-label={
                          m.active
                            ? accessLabel(m.id, data) === "Unassigned"
                              ? "Active, unassigned"
                              : "Active, assigned"
                            : "Disabled"
                        }
                      />
                      <div>
                        <strong>{m.name}</strong>
                        <p>
                          {!m.active ? "Disabled · " : ""}
                          {m.auth.kind === "none"
                            ? "No authentication"
                            : m.auth.kind === "bearer"
                              ? "Bearer token"
                              : "OAuth"}
                        </p>
                      </div>
                    </div>
                  </TableCell>
                  <TableCell>
                    {m.transport.kind === "stdio" ? "STDIO" : "Streamable HTTP"}
                  </TableCell>
                  <TableCell>{policyLabel(m)}</TableCell>
                  <TableCell className="access-cell">
                    {accessLabel(m.id, data)}
                  </TableCell>
                  <TableCell className="row-actions">
                    <Button
                      variant="outline"
                      disabled={actions.busy}
                      aria-label={`Manage ${m.name}`}
                      onClick={() => setEditing(m)}
                    >
                      Manage
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      ) : (
        <NoData
          title={search ? "No results" : "No MCPs yet"}
          description={
            search ? "Try another search." : "Add an MCP, then assign access."
          }
        />
      )}
      {editing !== undefined ? (
        <CatalogEditor
          key={editing?.id ?? "create"}
          initial={editing}
          data={data}
          actions={actions}
          onClose={() => setEditing(undefined)}
        />
      ) : null}
    </>
  );
}
