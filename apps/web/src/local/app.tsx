import { useEffect, useRef, useState } from "react";
import {
  Activity,
  List,
  LockKeyhole,
  LogOut,
  RefreshCw,
  TriangleAlert,
} from "lucide-react";
import { apiRequest, ApiFailure } from "@/lib/api";
import { empty } from "@/lib/decode";
import { cn } from "@/lib/utils";
import { GatewayMark } from "@/components/gateway-mark";
import { Feedback } from "@/components/forms";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Field, FieldGroup, FieldLabel } from "@/components/ui/field";
import { Spinner } from "@/components/ui/spinner";
import { Avatar, AvatarFallback } from "@/components/ui/avatar";
import { Separator } from "@/components/ui/separator";
import { Alert, AlertTitle, AlertDescription } from "@/components/ui/alert";
import { LocalCatalogScreen } from "./catalog";
import { Secrets } from "./secrets";
import { Status } from "./status";
import * as decode from "./contracts";
import type { LocalActions } from "./types";

const nav = [
  { id: "mcps", label: "MCPs", icon: List },
  { id: "secrets", label: "Secrets", icon: LockKeyhole },
  { id: "status", label: "Status", icon: Activity },
];
function route() {
  return nav.some((n) => n.id === location.hash.slice(1))
    ? location.hash.slice(1)
    : "mcps";
}
export function Brand() {
  return (
    <div className="brand">
      <GatewayMark />
      <div>
        <strong>MCP Gateway</strong>
        <p>This Mac</p>
      </div>
    </div>
  );
}
function Login({
  context,
  onLogin,
}: {
  context: decode.LocalContext;
  onLogin: () => Promise<void>;
}) {
  const [name, setName] = useState(""),
    [password, setPassword] = useState(""),
    [busy, setBusy] = useState(false),
    [error, setError] = useState<unknown>(null);
  async function submit() {
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      await apiRequest("/api/auth/login", decode.status, {
        method: "POST",
        csrfToken: context.csrfToken,
        body: { email: name, password },
        timeoutMs: 90_000,
      });
      await onLogin();
    } catch (e) {
      setError(e);
    } finally {
      setPassword("");
      setBusy(false);
    }
  }
  return (
    <div className="local-layout local-login">
      <aside className="local-sidebar">
        <Brand />
      </aside>
      <main className="local-login-main">
        <div className="local-login-form">
          <header>
            <h1>Sign in on this Mac</h1>
            <p>Connect to your enterprise MCP Gateway.</p>
          </header>
          <form
            onSubmit={(e) => {
              e.preventDefault();
              void submit();
            }}
          >
            <FieldGroup>
              <Field>
                <FieldLabel>Server</FieldLabel>
                <p className="muted break-value">{context.serverUrl}</p>
              </Field>
              <Field>
                <FieldLabel htmlFor="local-email">Email</FieldLabel>
                <Input
                  id="local-email"
                  placeholder="you@company.com"
                  type="email"
                  autoComplete="username"
                  maxLength={254}
                  required
                  disabled={busy}
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                />
              </Field>
              <Field>
                <FieldLabel htmlFor="local-password">Password</FieldLabel>
                <Input
                  id="local-password"
                  type="password"
                  autoComplete="current-password"
                  maxLength={128}
                  required
                  disabled={busy}
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                />
              </Field>
              <Feedback error={error} />
              <Button disabled={busy}>
                {busy ? <Spinner data-icon="inline-start" /> : null}Sign in
              </Button>
            </FieldGroup>
          </form>
          <p className="local-login-hint muted">
            Your password is never saved on this Mac.
          </p>
        </div>
      </main>
    </div>
  );
}
export function Freshness({
  online,
  seconds,
  valid,
}: {
  online: boolean;
  seconds: number;
  valid: boolean;
}) {
  if (!valid)
    return (
      <Alert variant="destructive" className="freshness expired">
        <TriangleAlert />
        <AlertTitle>Configuration unavailable</AlertTitle>
        <AlertDescription>
          Tools are blocked. Refresh online to load a complete configuration.
        </AlertDescription>
      </Alert>
    );
  if (!online)
    return (
      <Alert className="freshness offline">
        <TriangleAlert />
        <AlertTitle>Offline · Read-only</AlertTitle>
        <AlertDescription>
          Cached configuration expires in {decode.countdown(seconds)}. Reconnect
          to make changes.
        </AlertDescription>
      </Alert>
    );
  return (
    <Alert className="freshness connected" role="status">
      <AlertTitle>
        <span className="connection-dot" aria-hidden />
        Online
      </AlertTitle>
      <AlertDescription>
        Configuration expires in {decode.countdown(seconds)}
      </AlertDescription>
    </Alert>
  );
}
function Workspace({
  context,
  observed,
  onRefresh,
}: {
  context: decode.LocalContext;
  observed: number;
  onRefresh: () => Promise<void>;
}) {
  const generation = context.status.identity.generation ?? "";
  const [page, setPage] = useState(route),
    [now, setNow] = useState(Date.now),
    [error, setError] = useState<unknown>(null),
    [busy, setBusy] = useState(false),
    [reloadKey, setReloadKey] = useState(0);
  const [lifetime] = useState(() => ({ controller: new AbortController() }));
  useEffect(
    () => () => {
      lifetime.controller.abort();
      lifetime.controller = new AbortController();
    },
    [lifetime],
  );
  useEffect(() => {
    const reconnect = () => {
      void apiRequest("/api/refresh", decode.status, {
        method: "POST",
        csrfToken: context.csrfToken,
        body: { expectedGeneration: generation, input: null },
        signal: lifetime.controller.signal,
        timeoutMs: 90_000,
      })
        .catch(() => {
          /* Redacted native status reports the failed refresh. */
        })
        .finally(onRefresh);
    };
    window.addEventListener("online", reconnect);
    return () => window.removeEventListener("online", reconnect);
  }, [context.csrfToken, generation, lifetime, onRefresh]);
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 1000);
    const change = () => setPage(route());
    window.addEventListener("hashchange", change);
    return () => {
      clearInterval(timer);
      window.removeEventListener("hashchange", change);
    };
  }, []);
  const seconds = decode.remaining(context, observed, now),
    valid = seconds > 0,
    online = context.status.online && valid;
  const [read] = useState(
    () =>
      <T,>(
        path: string,
        decoder: (value: unknown) => T,
        signal?: AbortSignal,
      ) =>
        apiRequest(path, decoder, {
          endpointGeneration: generation,
          signal: signal
            ? AbortSignal.any([lifetime.controller.signal, signal])
            : lifetime.controller.signal,
          timeoutMs: 90_000,
        }),
  );
  async function write<T>(
    path: string,
    method: string,
    input: unknown,
    decoder: (value: unknown) => T,
  ): Promise<T> {
    const securityCancel =
      /^\/api\/(?:installations\/[^/]+|oauth\/flows\/[^/]+)\/cancel$/.test(
        path,
      );
    if (!online && !securityCancel)
      throw new ApiFailure(
        "offline",
        "Changes require a fresh online connection.",
        0,
      );
    return apiRequest(path, decoder, {
      method,
      csrfToken: context.csrfToken,
      body: { expectedGeneration: generation, input },
      signal: lifetime.controller.signal,
      timeoutMs: 90_000,
    });
  }
  function reload() {
    setError(null);
    setReloadKey((n) => n + 1);
    void onRefresh();
  }
  const actions: LocalActions = {
    generation,
    context,
    online,
    read,
    write,
    reload,
  };
  async function refresh() {
    if (busy) return;
    setBusy(true);
    setError(null);
    try {
      await apiRequest("/api/refresh", decode.status, {
        method: "POST",
        csrfToken: context.csrfToken,
        body: { expectedGeneration: generation, input: null },
        signal: lifetime.controller.signal,
        timeoutMs: 90_000,
      });
      reload();
    } catch (e) {
      setError(e);
      await onRefresh();
    } finally {
      setBusy(false);
    }
  }
  async function logout() {
    setBusy(true);
    setError(null);
    try {
      await apiRequest("/api/auth/logout", empty, {
        method: "POST",
        csrfToken: context.csrfToken,
        endpointGeneration: generation,
        signal: lifetime.controller.signal,
        timeoutMs: 90_000,
      });
      location.replace("/");
    } catch (e) {
      setError(e);
      await onRefresh();
    } finally {
      setBusy(false);
    }
  }
  const title =
    page === "secrets"
      ? "Your secrets"
      : page === "status"
        ? "Connection status"
        : "Your MCPs";
  const description =
    page === "secrets"
      ? "Encrypted on the server. Values are write-only."
      : page === "status"
        ? "Configuration and authentication on this Mac."
        : "Choose what your coding agent can use.";
  return (
    <div className="local-layout">
      <aside className="local-sidebar">
        <Brand />
        <nav aria-label="Local management">
          {nav.map((n) => (
            <a
              key={n.id}
              href={`#${n.id}`}
              className={cn("nav-link", page === n.id && "selected")}
              aria-current={page === n.id ? "page" : undefined}
            >
              <n.icon aria-hidden />
              {n.label}
            </a>
          ))}
        </nav>
        <div className="account">
          <div className="account-identity">
            <Avatar>
              <AvatarFallback>
                {context.status.identity.name?.slice(0, 1).toUpperCase() ?? "U"}
              </AvatarFallback>
            </Avatar>
            <strong>{context.status.identity.name ?? "Signed in"}</strong>
          </div>
          <Separator />
          <Button variant="ghost" onClick={() => void logout()} disabled={busy}>
            <LogOut data-icon="inline-start" />
            Sign out
          </Button>
        </div>
      </aside>
      <main className="local-main">
        {page !== "secrets" ? (
          <header className="page-header">
            <div>
              <h1>{title}</h1>
              <p>{description}</p>
            </div>
            {page !== "secrets" ? (
              <Button
                variant="outline"
                disabled={busy}
                onClick={() => void refresh()}
              >
                {busy ? (
                  <Spinner data-icon="inline-start" />
                ) : (
                  <RefreshCw data-icon="inline-start" />
                )}
                Refresh
              </Button>
            ) : null}
          </header>
        ) : null}
        {page !== "secrets" ? (
          <Freshness online={online} seconds={seconds} valid={valid} />
        ) : null}
        <Feedback
          error={error}
          reload={() => {
            void refresh();
          }}
        />
        <div key={`${page}:${reloadKey}`}>
          {page === "mcps" ? (
            <LocalCatalogScreen actions={actions} valid={valid} />
          ) : page === "secrets" ? (
            <Secrets
              actions={actions}
              freshness={
                <Freshness online={online} seconds={seconds} valid={valid} />
              }
            />
          ) : (
            <Status context={context} valid={valid} />
          )}
        </div>
      </main>
    </div>
  );
}
export function LocalApp() {
  const sequence = useRef(0);
  const [snapshot, setSnapshot] = useState<{
      context: decode.LocalContext;
      observed: number;
    } | null>(null),
    [error, setError] = useState<unknown>(null);
  const [refresh] = useState(() => async () => {
    const request = ++sequence.current;
    try {
      const context = await apiRequest("/api/context", decode.context, {
        timeoutMs: 90_000,
      });
      if (request === sequence.current) {
        setSnapshot({ context, observed: Date.now() });
        setError(null);
      }
    } catch (e) {
      if (request === sequence.current) {
        setError(e);
        setSnapshot(null);
      }
    }
  });
  useEffect(() => {
    let active = true;
    const abort = new AbortController();
    async function poll() {
      const request = ++sequence.current;
      try {
        const context = await apiRequest("/api/context", decode.context, {
          signal: abort.signal,
          timeoutMs: 90_000,
        });
        if (active && request === sequence.current) {
          setSnapshot({ context, observed: Date.now() });
          setError(null);
        }
      } catch (e) {
        if (active && request === sequence.current) {
          setError(e);
          setSnapshot(null);
        }
      }
    }
    void poll();
    let polling = false;
    const wake = () => {
      if (document.visibilityState !== "hidden" && !polling) {
        polling = true;
        void poll().finally(() => {
          polling = false;
        });
      }
    };
    const timer = setInterval(wake, 5000);
    window.addEventListener("online", wake);
    window.addEventListener("focus", wake);
    document.addEventListener("visibilitychange", wake);
    return () => {
      active = false;
      abort.abort();
      clearInterval(timer);
      window.removeEventListener("online", wake);
      window.removeEventListener("focus", wake);
      document.removeEventListener("visibilitychange", wake);
    };
  }, []);
  if (error)
    return (
      <main className="sign-in">
        <Brand />
        <Feedback error={error} />
        <Button onClick={() => location.replace("/")}>Reload local page</Button>
      </main>
    );
  if (!snapshot)
    return (
      <main className="sign-in">
        <Brand />
        <Spinner />
        <p>Loading local configuration</p>
      </main>
    );
  const { context, observed } = snapshot;
  if (context.status.identity.state === "server_mismatch")
    return (
      <main className="sign-in">
        <Brand />
        <h1>Server selection changed</h1>
        <p>Start the local UI for the selected server again.</p>
      </main>
    );
  if (
    context.status.identity.state !== "authenticated" ||
    !context.status.identity.generation
  )
    return <Login context={context} onLogin={refresh} />;
  return (
    <Workspace
      key={context.status.identity.generation}
      context={context}
      observed={observed}
      onRefresh={refresh}
    />
  );
}
