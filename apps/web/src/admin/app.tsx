import { useEffect, useState } from "react";
import {
  FileText,
  LockKeyhole,
  UserRound,
  UsersRound,
  LogOut,
} from "lucide-react";
import type { UiContext } from "@/generated/UiContext";
import { Button } from "@/components/ui/button";
import { GatewayMark } from "@/components/gateway-mark";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Separator } from "@/components/ui/separator";
import { Skeleton } from "@/components/ui/skeleton";
import { Spinner } from "@/components/ui/spinner";
import { Input } from "@/components/ui/input";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Feedback } from "@/components/forms";
import { apiRequest, ApiFailure } from "@/lib/api";
import * as decode from "@/lib/decode";
import { cn } from "@/lib/utils";
import { Catalog } from "./catalog";
import { Access } from "./access";
import { Users, Groups } from "./identities";
import type { AdminData, Actions } from "./types";

const navigation = [
  { id: "catalog", label: "Catalog", icon: FileText },
  { id: "access", label: "Access", icon: LockKeyhole },
  { id: "users", label: "Users", icon: UserRound },
  { id: "groups", label: "Groups", icon: UsersRound },
];
function route() {
  const id = window.location.hash.slice(1);
  return navigation.some((n) => n.id === id) ? id : "catalog";
}
function Brand() {
  return (
    <div className="brand">
      <GatewayMark />
      <div>
        <strong>MCP Gateway</strong>
        <p>Administration</p>
      </div>
    </div>
  );
}
function SignIn({
  context,
  onLogin,
}: {
  context: UiContext;
  onLogin: (context: UiContext) => void;
}) {
  const [name, setName] = useState("");
  const [password, setPassword] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<unknown>(null);
  async function submit() {
    setBusy(true);
    setError(null);
    try {
      onLogin(
        await apiRequest("/api/auth/login", decode.context, {
          method: "POST",
          csrfToken: context.csrfToken ?? undefined,
          body: { username: name, password },
        }),
      );
    } catch (e) {
      setError(e);
    } finally {
      setPassword("");
      setBusy(false);
    }
  }
  return (
    <main className="sign-in">
      <Brand />
      <h1>Sign in</h1>
      <form
        onSubmit={(e) => {
          e.preventDefault();
          void submit();
        }}
      >
        <FieldGroup>
          <Field>
            <FieldLabel htmlFor="username">Username</FieldLabel>
            <Input
              id="username"
              value={name}
              onChange={(e) => setName(e.target.value)}
              autoComplete="username"
              required
              maxLength={64}
              disabled={busy}
            />
          </Field>
          <Field>
            <FieldLabel htmlFor="password">Password</FieldLabel>
            <Input
              id="password"
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              autoComplete="current-password"
              required
              maxLength={128}
              disabled={busy}
            />
          </Field>
          <Feedback error={error} />
          <Button disabled={busy || !context.csrfToken}>
            {busy ? <Spinner data-icon="inline-start" /> : null}Sign in
          </Button>
        </FieldGroup>
      </form>
    </main>
  );
}
function mutation(path: string, method: string, value: unknown): unknown {
  if (
    path.includes("/password") ||
    (path.includes("/groups/") && method === "DELETE")
  )
    return decode.empty(value);
  if (path.startsWith("/api/admin/users")) return decode.user(value);
  if (path.startsWith("/api/admin/groups")) return decode.group(value);
  if (path === "/api/admin/assignments" && method === "POST")
    return decode.assignment(value);
  return decode.revision(value);
}
function Administration({
  context,
  onSessionChange,
}: {
  context: UiContext;
  onSessionChange: () => void;
}) {
  const [page, setPage] = useState(route);
  const [data, setData] = useState<AdminData | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);
  const [generation, setGeneration] = useState(0);
  const [notice, setNotice] = useState("");
  useEffect(() => {
    const change = () => setPage(route());
    window.addEventListener("hashchange", change);
    return () => window.removeEventListener("hashchange", change);
  }, []);
  useEffect(() => {
    const abort = new AbortController();
    async function load() {
      try {
        const [config, users, groups] = await Promise.all([
          apiRequest("/api/admin/configuration", decode.configuration, {
            signal: abort.signal,
          }),
          apiRequest("/api/admin/users", (v) => decode.array(v, decode.user), {
            signal: abort.signal,
          }),
          apiRequest(
            "/api/admin/groups",
            (v) => decode.array(v, decode.group),
            { signal: abort.signal },
          ),
        ]);
        if (!abort.signal.aborted) {
          setData({ config, users, groups });
          setError(null);
        }
      } catch (e) {
        if (!abort.signal.aborted) {
          if (e instanceof ApiFailure && e.status === 401) onSessionChange();
          else setError(e);
        }
      } finally {
        if (!abort.signal.aborted) setLoading(false);
      }
    }
    void load();
    return () => abort.abort();
  }, [generation, onSessionChange]);
  function reload() {
    setLoading(true);
    setData(null);
    setError(null);
    setGeneration((old) => old + 1);
  }
  async function refreshData() {
    const [config, users, groups] = await Promise.all([
      apiRequest("/api/admin/configuration", decode.configuration),
      apiRequest("/api/admin/users", (v) => decode.array(v, decode.user)),
      apiRequest("/api/admin/groups", (v) => decode.array(v, decode.group)),
    ]);
    setData({ config, users, groups });
  }
  async function execute(path: string, method: string, body: unknown) {
    if (busy) return false;
    setBusy(true);
    setError(null);
    setNotice("");
    let confirmed = false;
    try {
      await apiRequest(path, (v) => mutation(path, method, v), {
        method,
        body,
        csrfToken: context.csrfToken ?? undefined,
      });
      confirmed = true;
      setNotice("Saved");
      await refreshData();
      return true;
    } catch (e) {
      if (e instanceof ApiFailure && e.status === 401) onSessionChange();
      else {
        if (confirmed) setData(null);
        setError(
          confirmed
            ? new ApiFailure(
                "offline",
                "The change was saved, but current state could not be loaded. Reload before making another change.",
                0,
              )
            : e,
        );
      }
      return confirmed;
    } finally {
      setBusy(false);
    }
  }
  async function logout() {
    setBusy(true);
    try {
      await apiRequest("/api/auth/logout", decode.empty, {
        method: "POST",
        csrfToken: context.csrfToken ?? undefined,
      });
      onSessionChange();
    } catch (e) {
      if (e instanceof ApiFailure && e.status === 401) onSessionChange();
      else setError(e);
    } finally {
      setBusy(false);
    }
  }
  const actions: Actions = { execute, reload, busy, error };
  return (
    <div className="admin-layout">
      <aside className="admin-sidebar">
        <Brand />
        <nav aria-label="Administration">
          {navigation.map((n) => (
            <a
              key={n.id}
              href={`#${n.id}`}
              aria-current={page === n.id ? "page" : undefined}
              className={cn("nav-link", page === n.id && "selected")}
            >
              <n.icon aria-hidden />
              {n.label}
            </a>
          ))}
        </nav>
        <div className="account">
          <Separator />
          <div className="account-identity">
            <Avatar size="lg">
              <AvatarFallback>
                {context.user?.username.slice(0, 1).toUpperCase()}
              </AvatarFallback>
            </Avatar>
            <div>
              <strong>{context.user?.username}</strong>
              <p>Administrator</p>
            </div>
          </div>
          <Button
            variant="ghost"
            onClick={() => {
              void logout();
            }}
            disabled={busy}
          >
            <LogOut data-icon="inline-start" />
            Sign out
          </Button>
        </div>
      </aside>
      <main className="admin-main">
        <Feedback error={error} reload={reload} />
        {notice ? (
          <p role="status" className="save-notice">
            {notice}
          </p>
        ) : null}
        {loading ? (
          <div aria-label="Loading administration" className="loading-page">
            <Skeleton className="h-12 w-64" />
            <Skeleton className="h-60 w-full" />
          </div>
        ) : data ? (
          <div key={generation}>
            {page === "catalog" ? (
              <Catalog data={data} actions={actions} />
            ) : page === "access" ? (
              <Access data={data} actions={actions} />
            ) : page === "users" ? (
              <Users data={data} actions={actions} />
            ) : (
              <Groups data={data} actions={actions} />
            )}
          </div>
        ) : null}
      </main>
    </div>
  );
}
export function AdminApp() {
  const [context, setContext] = useState<UiContext | null>(null);
  const [error, setError] = useState<unknown>(null);
  const [request, setRequest] = useState(0);
  useEffect(() => {
    const abort = new AbortController();
    void apiRequest("/api/context", decode.context, { signal: abort.signal })
      .then((value) => {
        if (!abort.signal.aborted) {
          setContext(value);
          setError(null);
        }
      })
      .catch((e) => {
        if (!abort.signal.aborted) setError(e);
      });
    return () => abort.abort();
  }, [request]);
  // Stable callback avoids refetching administration whenever unrelated context rendering changes.
  const [sessionChange] = useState(() => () => {
    setContext(null);
    setRequest((old) => old + 1);
  });
  if (error)
    return (
      <main className="sign-in">
        <Brand />
        <Feedback
          error={error}
          reload={() => {
            setError(null);
            sessionChange();
          }}
        />
      </main>
    );
  if (!context)
    return (
      <main className="sign-in">
        <Brand />
        <Spinner />
        <p>Loading</p>
      </main>
    );
  if (!context.user) return <SignIn context={context} onLogin={setContext} />;
  if (context.user.role !== "admin")
    return (
      <main className="sign-in">
        <Brand />
        <h1>Administrator access required</h1>
        <Button
          onClick={() => {
            void apiRequest("/api/auth/logout", decode.empty, {
              method: "POST",
              csrfToken: context.csrfToken ?? undefined,
            })
              .then(sessionChange)
              .catch(setError);
          }}
        >
          Sign out
        </Button>
      </main>
    );
  return <Administration context={context} onSessionChange={sessionChange} />;
}
