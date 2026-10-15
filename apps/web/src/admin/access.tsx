import { useState } from "react";
import type { GrantTarget } from "@/generated/GrantTarget";
import type { Assignment } from "@/generated/Assignment";
import { FieldGroup, FieldDescription } from "@/components/ui/field";
import { Button } from "@/components/ui/button";
import {
  Table,
  TableHeader,
  TableBody,
  TableRow,
  TableHead,
  TableCell,
} from "@/components/ui/table";
import { Choice, Editor, Feedback, NoData } from "@/components/forms";
import type { AdminData, Actions } from "./types";

export function targetLabel(target: GrantTarget, data: AdminData): string {
  switch (target.kind) {
    case "all":
      return "All users";
    case "user":
      return data.users.find((u) => u.id === target.id)?.name ?? "Unknown user";
    case "group":
      return (
        data.groups.find((g) => g.id === target.id)?.name ?? "Unknown group"
      );
  }
}
export function accessLabel(mcpId: string, data: AdminData): string {
  return (
    data.config.assignments
      .filter((a) => a.mcpId === mcpId)
      .map((a) => targetLabel(a.target, data))
      .join(", ") || "Unassigned"
  );
}
export function AssignmentEditor({
  data,
  actions,
  onClose,
  mcpId,
}: {
  data: AdminData;
  actions: Actions;
  onClose: () => void;
  mcpId?: string;
}) {
  const [mcp, setMcp] = useState(mcpId ?? data.config.mcps[0]?.id ?? "");
  const [kind, setKind] = useState("all");
  const [targetId, setTargetId] = useState("");
  const [revision] = useState(data.config.revision);
  const choices =
    kind === "user"
      ? data.users.map((u) => ({
          value: u.id,
          label: `${u.name} (${u.email})`,
        }))
      : data.groups.map((g) => ({ value: g.id, label: g.name }));
  async function submit() {
    const target: GrantTarget =
      kind === "all"
        ? { kind: "all" }
        : kind === "user"
          ? { kind: "user", id: targetId }
          : { kind: "group", id: targetId };
    if (
      await actions.execute("/api/admin/assignments", "POST", {
        expectedRevision: revision,
        mcpId: mcp,
        target,
      })
    )
      onClose();
  }
  return (
    <Editor
      title="Add assignment"
      description="Grants add access. Users still choose what to enable."
      onClose={onClose}
      busy={actions.busy}
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          void submit();
        }}
        className="editor-form"
      >
        <div className="editor-body">
          <Feedback error={actions.error} reload={actions.reload} />
          <FieldGroup>
            <Choice
              label="MCP"
              value={mcp}
              onChange={setMcp}
              choices={data.config.mcps.map((m) => ({
                value: m.id,
                label: m.name,
              }))}
            />
            <Choice
              label="Assigned to"
              value={kind}
              onChange={(v) => {
                setKind(v);
                setTargetId("");
              }}
              choices={[
                { value: "all", label: "All users" },
                { value: "user", label: "User" },
                { value: "group", label: "Group" },
              ]}
            />
            {kind !== "all" ? (
              <Choice
                label={kind === "user" ? "Select user" : "Select group"}
                value={targetId}
                onChange={setTargetId}
                choices={choices}
              />
            ) : null}
          </FieldGroup>
        </div>
        <div className="editor-actions">
          <Button
            type="button"
            variant="outline"
            onClick={onClose}
            disabled={actions.busy}
          >
            Cancel
          </Button>
          <Button
            disabled={actions.busy || !mcp || (kind !== "all" && !targetId)}
          >
            Add assignment
          </Button>
        </div>
      </form>
    </Editor>
  );
}
export function AssignmentRows({
  data,
  actions,
  assignments,
}: {
  data: AdminData;
  actions: Actions;
  assignments: Assignment[];
}) {
  return assignments.length ? (
    <div className="data-table">
      <Table>
        <TableHeader>
          <TableRow>
            <TableHead scope="col">MCP</TableHead>
            <TableHead scope="col">Assigned to</TableHead>
            <TableHead scope="col">
              <span className="sr-only">Actions</span>
            </TableHead>
          </TableRow>
        </TableHeader>
        <TableBody>
          {assignments.map((a) => (
            <TableRow key={a.id}>
              <TableCell>
                {data.config.mcps.find((m) => m.id === a.mcpId)?.name ??
                  "Unknown MCP"}
              </TableCell>
              <TableCell>{targetLabel(a.target, data)}</TableCell>
              <TableCell className="row-actions">
                <Button
                  variant="outline"
                  disabled={actions.busy}
                  aria-label={`Remove assignment ${targetLabel(a.target, data)}`}
                  onClick={() => {
                    void actions.execute(
                      `/api/admin/assignments/${a.id}`,
                      "DELETE",
                      { expectedRevision: data.config.revision },
                    );
                  }}
                >
                  Remove assignment
                </Button>
              </TableCell>
            </TableRow>
          ))}
        </TableBody>
      </Table>
    </div>
  ) : (
    <NoData title="No assignments yet" />
  );
}
export function Access({
  data,
  actions,
}: {
  data: AdminData;
  actions: Actions;
}) {
  const [adding, setAdding] = useState(false);
  return (
    <>
      <header className="page-header">
        <div>
          <h1>Access</h1>
          <p>Grants add access. Removing the last grant clears enablement.</p>
        </div>
        <Button
          disabled={!data.config.mcps.length || actions.busy}
          onClick={() => setAdding(true)}
        >
          Add assignment
        </Button>
      </header>
      <FieldDescription>
        Assign an MCP to a user, group, or all users.
      </FieldDescription>
      <AssignmentRows
        data={data}
        actions={actions}
        assignments={data.config.assignments}
      />
      {adding ? (
        <AssignmentEditor
          data={data}
          actions={actions}
          onClose={() => setAdding(false)}
        />
      ) : null}
    </>
  );
}
