import { useState } from "react";
import { Plus, Trash2 } from "lucide-react";
import type { UserInfo } from "@/generated/UserInfo";
import type { GroupInfo } from "@/generated/GroupInfo";
import type { Role } from "@/generated/Role";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import {
  Field,
  FieldLabel,
  FieldGroup,
  FieldSet,
  FieldLegend,
  FieldContent,
  FieldDescription,
} from "@/components/ui/field";
import {
  Table,
  TableHeader,
  TableBody,
  TableRow,
  TableHead,
  TableCell,
} from "@/components/ui/table";
import {
  AlertDialog,
  AlertDialogContent,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogCancel,
  AlertDialogAction,
} from "@/components/ui/alert-dialog";
import {
  Editor,
  TextField,
  Choice,
  Feedback,
  NoData,
} from "@/components/forms";
import type { Actions, AdminData } from "./types";

export function ConfirmDelete({
  title,
  description,
  onConfirm,
  onClose,
  busy,
}: {
  title: string;
  description: string;
  onConfirm: () => void;
  onClose: () => void;
  busy: boolean;
}) {
  return (
    <AlertDialog
      open
      onOpenChange={(open) => {
        if (!open && !busy) onClose();
      }}
    >
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{title}</AlertDialogTitle>
          <AlertDialogDescription>{description}</AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel disabled={busy}>Cancel</AlertDialogCancel>
          <AlertDialogAction
            variant="destructive"
            disabled={busy}
            onClick={(e) => {
              e.preventDefault();
              onConfirm();
            }}
          >
            Delete
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
function UserEditor({
  initial,
  data,
  actions,
  onClose,
}: {
  initial: UserInfo | null;
  data: AdminData;
  actions: Actions;
  onClose: () => void;
}) {
  const [name, setName] = useState(initial?.username ?? "");
  const [password, setPassword] = useState("");
  const [role, setRole] = useState<Role>(initial?.role ?? "user");
  const [active, setActive] = useState(initial?.active ?? true);
  const [revision] = useState(data.config.revision);
  const [resetting, setResetting] = useState(false);
  async function submit() {
    const ok = initial
      ? await actions.execute(`/api/admin/users/${initial.id}`, "PUT", {
          expectedRevision: revision,
          role,
          active,
        })
      : await actions.execute("/api/admin/users", "POST", {
          expectedRevision: revision,
          username: name,
          password,
          role,
        });
    setPassword("");
    if (ok) onClose();
  }
  async function reset() {
    try {
      if (
        initial &&
        (await actions.execute(
          `/api/admin/users/${initial.id}/password`,
          "PUT",
          { expectedRevision: revision, password },
        ))
      )
        onClose();
    } finally {
      setPassword("");
    }
  }
  return (
    <Editor
      title={initial?.username ?? "Create user"}
      description="Account and password changes revoke this user's sessions."
      onClose={onClose}
      busy={actions.busy}
    >
      <form
        className="editor-form"
        onSubmit={(e) => {
          e.preventDefault();
          void (resetting ? reset() : submit());
        }}
      >
        <div className="editor-body">
          <Feedback error={actions.error} reload={actions.reload} />
          <FieldGroup>
            {resetting ? (
              <TextField
                label="New password"
                type="password"
                value={password}
                onChange={setPassword}
                required
                maxLength={128}
                description="12–128 bytes."
              />
            ) : (
              <>
                <TextField
                  label="Username"
                  value={name}
                  onChange={setName}
                  required
                  disabled={Boolean(initial)}
                  maxLength={64}
                />
                {!initial ? (
                  <TextField
                    label="Password"
                    type="password"
                    value={password}
                    onChange={setPassword}
                    required
                    maxLength={128}
                    description="12–128 bytes."
                  />
                ) : null}
                <Choice
                  label="Role"
                  value={role}
                  onChange={(v) => {
                    if (v === "user" || v === "admin") setRole(v);
                  }}
                  choices={[
                    { value: "user", label: "User" },
                    { value: "admin", label: "Administrator" },
                  ]}
                />
                {initial ? (
                  <Field orientation="horizontal">
                    <Checkbox
                      id="user-active"
                      checked={active}
                      onCheckedChange={(v) => setActive(v === true)}
                    />
                    <FieldLabel htmlFor="user-active">Active</FieldLabel>
                  </Field>
                ) : null}
              </>
            )}
          </FieldGroup>
        </div>
        <div className="editor-actions">
          {initial && !resetting ? (
            <Button
              type="button"
              variant="ghost"
              onClick={() => setResetting(true)}
              disabled={actions.busy}
            >
              Reset password
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
            {resetting
              ? "Reset password"
              : initial
                ? "Save user"
                : "Create user"}
          </Button>
        </div>
      </form>
    </Editor>
  );
}
export function Users({
  data,
  actions,
}: {
  data: AdminData;
  actions: Actions;
}) {
  const [editing, setEditing] = useState<UserInfo | null | undefined>();
  return (
    <>
      <header className="page-header">
        <h1>Users</h1>
        <Button onClick={() => setEditing(null)} disabled={actions.busy}>
          <Plus data-icon="inline-start" />
          Create user
        </Button>
      </header>
      {data.users.length ? (
        <div className="data-table">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead scope="col">Username</TableHead>
                <TableHead scope="col">Role</TableHead>
                <TableHead scope="col">Status</TableHead>
                <TableHead scope="col">
                  <span className="sr-only">Actions</span>
                </TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {data.users.map((u) => (
                <TableRow key={u.id}>
                  <TableCell>
                    <strong>{u.username}</strong>
                  </TableCell>
                  <TableCell>
                    {u.role === "admin" ? "Administrator" : "User"}
                  </TableCell>
                  <TableCell>{u.active ? "Active" : "Disabled"}</TableCell>
                  <TableCell className="row-actions">
                    <Button
                      variant="outline"
                      aria-label={`Edit user ${u.username}`}
                      onClick={() => setEditing(u)}
                      disabled={actions.busy}
                    >
                      Edit
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      ) : (
        <NoData title="No users yet" />
      )}
      {editing !== undefined ? (
        <UserEditor
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
function GroupEditor({
  initial,
  data,
  actions,
  onClose,
}: {
  initial: GroupInfo | null;
  data: AdminData;
  actions: Actions;
  onClose: () => void;
}) {
  const [name, setName] = useState(initial?.name ?? "");
  const [members, setMembers] = useState(initial?.memberIds ?? []);
  const [revision] = useState(data.config.revision);
  const [deleting, setDeleting] = useState(false);
  async function submit() {
    if (
      await actions.execute(
        initial ? `/api/admin/groups/${initial.id}` : "/api/admin/groups",
        initial ? "PUT" : "POST",
        { expectedRevision: revision, name, memberIds: members },
      )
    )
      onClose();
  }
  return (
    <Editor
      title={initial?.name ?? "Create group"}
      description="Choose who belongs to this group."
      onClose={onClose}
      busy={actions.busy}
    >
      <form
        className="editor-form"
        onSubmit={(e) => {
          e.preventDefault();
          void submit();
        }}
      >
        <div className="editor-body">
          <Feedback error={actions.error} reload={actions.reload} />
          <FieldGroup>
            <TextField
              label="Group name"
              value={name}
              onChange={setName}
              required
              maxLength={128}
            />
            <FieldSet>
              <FieldLegend variant="label">Members</FieldLegend>
              <FieldGroup className="member-list">
                {data.users.map((u) => (
                  <Field
                    key={u.id}
                    orientation="horizontal"
                    className="member-row"
                  >
                    <Checkbox
                      id={`member-${u.id}`}
                      checked={members.includes(u.id)}
                      onCheckedChange={(v) =>
                        setMembers((old) =>
                          v === true
                            ? [...old, u.id]
                            : old.filter((id) => id !== u.id),
                        )
                      }
                    />
                    <FieldContent>
                      <FieldLabel htmlFor={`member-${u.id}`}>
                        {u.username}
                      </FieldLabel>
                      <FieldDescription>
                        {u.role === "admin" ? "Administrator" : "User"}
                        {u.active ? "" : " · Disabled"}
                      </FieldDescription>
                    </FieldContent>
                  </Field>
                ))}
                {!data.users.length ? <NoData title="No users yet" /> : null}
              </FieldGroup>
            </FieldSet>
          </FieldGroup>
        </div>
        <div className="editor-actions">
          {initial ? (
            <Button
              type="button"
              variant="destructive"
              onClick={() => setDeleting(true)}
              disabled={actions.busy}
            >
              <Trash2 data-icon="inline-start" />
              Delete group
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
          <Button disabled={actions.busy}>Save group</Button>
        </div>
      </form>
      {deleting && initial ? (
        <ConfirmDelete
          title={`Delete ${initial.name}?`}
          description="Group assignments and memberships will be removed. Users losing their last grant are disabled for those MCPs."
          busy={actions.busy}
          onClose={() => setDeleting(false)}
          onConfirm={() => {
            void actions
              .execute(`/api/admin/groups/${initial.id}`, "DELETE", {
                expectedRevision: revision,
              })
              .then((ok) => {
                if (ok) onClose();
                else setDeleting(false);
              });
          }}
        />
      ) : null}
    </Editor>
  );
}
export function Groups({
  data,
  actions,
}: {
  data: AdminData;
  actions: Actions;
}) {
  const [editing, setEditing] = useState<GroupInfo | null | undefined>();
  return (
    <>
      <header className="page-header">
        <div>
          <h1>Groups</h1>
          <p>Group membership adds access to assigned MCPs.</p>
        </div>
        <Button onClick={() => setEditing(null)} disabled={actions.busy}>
          <Plus data-icon="inline-start" />
          Create group
        </Button>
      </header>
      {data.groups.length ? (
        <div className="data-table">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead scope="col">Group</TableHead>
                <TableHead scope="col">Members</TableHead>
                <TableHead scope="col">
                  <span className="sr-only">Actions</span>
                </TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {data.groups.map((g) => (
                <TableRow key={g.id}>
                  <TableCell>
                    <strong>{g.name}</strong>
                  </TableCell>
                  <TableCell>
                    {g.memberIds
                      .map(
                        (id) =>
                          data.users.find((u) => u.id === id)?.username ??
                          "Unknown user",
                      )
                      .join(", ") || "No members"}
                  </TableCell>
                  <TableCell className="row-actions">
                    <Button
                      variant="outline"
                      aria-label={`Edit group ${g.name}`}
                      onClick={() => setEditing(g)}
                      disabled={actions.busy}
                    >
                      Edit
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      ) : (
        <NoData title="No groups yet" />
      )}
      {editing !== undefined ? (
        <GroupEditor
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
