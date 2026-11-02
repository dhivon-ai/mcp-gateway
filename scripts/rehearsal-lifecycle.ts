/** Private rehearsal ownership; never takes a deployment URL or caller-selected project. */
import assert from "node:assert/strict";
import { spawn, spawnSync, type ChildProcess } from "node:child_process";
import { createHash, randomUUID } from "node:crypto";
import {
  closeSync,
  copyFileSync,
  existsSync,
  fsyncSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  openSync,
  readFileSync,
  realpathSync,
  renameSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, join, resolve } from "node:path";
import { setTimeout as pause } from "node:timers/promises";
import { RehearsalLock } from "./rehearsal-lock.ts";

type Owner = { pid: number; identity: string };
type OwnedChild = {
  id: string;
  binary: string;
  args: string[];
  owner: Owner | null;
  phase?: "intent" | "spawned" | "exited" | "not_spawned";
};
type Step = { ok: boolean; error?: string };
type Journal = {
  schema: 1;
  directory: string;
  project: string;
  cli: string;
  state: string;
  cliHash: string;
  files: Record<string, string>;
  supervisor: Owner;
  details?: Record<string, string>;
  children: OwnedChild[];
  workflowSucceeded: boolean;
  interrupted: string | null;
  cleanup: Record<string, Step>;
  status: "running" | "cleaning" | "clean" | "recovery_required";
  cleanupHistory: Array<{
    at: string;
    status: string;
    steps: Record<string, Step>;
  }>;
};
export function digest(path: string) {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}
function identity(pid: number): string {
  const r = spawnSync(
    "ps",
    ["-p", String(pid), "-o", "lstart=", "-o", "command="],
    { encoding: "utf8", timeout: 2000 },
  );
  if (r.status === 1 && !r.stdout.trim()) return "";
  assert.equal(r.status, 0, "Cannot establish exact child identity");
  return r.stdout.trim();
}
function owner(): Owner {
  return { pid: process.pid, identity: identity(process.pid) };
}
function active(value: Owner) {
  return identity(value.pid) === value.identity;
}
function atomic(path: string, value: unknown) {
  const temporary = `${path}.tmp`;
  const fd = openSync(temporary, "w", 0o600);
  try {
    writeFileSync(fd, JSON.stringify(value, null, 2) + "\n");
    fsyncSync(fd);
  } finally {
    closeSync(fd);
  }
  renameSync(temporary, path);
  const directory = openSync(dirname(path), "r");
  try {
    fsyncSync(directory);
  } finally {
    closeSync(directory);
  }
}
function privatePath(path: string) {
  const info = lstatSync(path);
  assert(
    !info.isSymbolicLink() &&
      (info.mode & 0o077) === 0 &&
      info.uid === process.getuid?.(),
    "Recovery requires private files owned by this user",
  );
}

export class Rehearsal {
  readonly abort = new AbortController();
  readonly log: number;
  readonly journal: Journal;
  private handles = new Map<string, ChildProcess>();
  private cleaning?: Promise<boolean>;
  private handlers: Array<[NodeJS.Signals, () => void]> = [];
  private lock: RehearsalLock;
  private closed = false;
  private constructor(journal: Journal, heldLock?: RehearsalLock) {
    this.journal = journal;
    this.lock = heldLock ?? new RehearsalLock(journal.directory);
    try {
      this.log = openSync(join(journal.directory, "private.log"), "a", 0o600);
    } catch (error) {
      this.lock.close();
      throw error;
    }
  }
  close() {
    if (this.closed) return;
    this.closed = true;
    for (const [signal, handler] of this.handlers)
      process.removeListener(signal, handler);
    try {
      closeSync(this.log);
    } finally {
      this.lock.close();
    }
  }
  static create(cache: string, cli: string) {
    const directory = mkdtempSync(join(realpathSync(cache), "https-oauth-"));
    const creatingCli = join(directory, "dhivon-mcp-gateway");
    copyFileSync(cli, creatingCli);
    mkdirSync(join(directory, "native"), { mode: 0o700 });
    const j: Journal = {
      schema: 1,
      directory,
      project: `gw-https-${randomUUID().replaceAll("-", "")}`,
      cli: creatingCli,
      state: realpathSync(join(directory, "native")),
      cliHash: digest(creatingCli),
      files: {},
      supervisor: owner(),
      children: [],
      workflowSucceeded: false,
      interrupted: null,
      cleanup: {},
      cleanupHistory: [],
      status: "running",
    };
    atomic(join(directory, "ownership.json"), j);
    return new Rehearsal(j);
  }
  static recover(cache: string, directory: string) {
    directory = realpathSync(directory);
    assert.equal(
      dirname(directory),
      realpathSync(cache),
      "Recovery directory must be directly inside this execution's owned cache",
    );
    assert(/^https-oauth-[a-zA-Z0-9]+$/.test(basename(directory)));
    privatePath(directory);
    privatePath(join(directory, "ownership.json"));
    assert.equal(
      realpathSync(join(directory, "native")),
      join(directory, "native"),
    );
    assert.equal(
      realpathSync(join(directory, "dhivon-mcp-gateway")),
      join(directory, "dhivon-mcp-gateway"),
    );
    // Acquire before reading the snapshot: a waiter cannot overwrite a predecessor's final journal.
    const heldLock = new RehearsalLock(directory);
    try {
      const j = JSON.parse(
        readFileSync(join(directory, "ownership.json"), "utf8"),
      ) as Journal;
      assert(
        j.schema === 1 &&
          j.directory === directory &&
          /^gw-https-[a-f0-9]{32}$/.test(j.project),
      );
      assert(
        j.cli === join(directory, "dhivon-mcp-gateway") &&
          j.state === join(directory, "native"),
      );
      assert(
        typeof j.cliHash === "string" && j.cliHash === digest(j.cli),
        "Retain the exact creating binary",
      );
      assert(
        j.supervisor &&
          Number.isSafeInteger(j.supervisor.pid) &&
          typeof j.supervisor.identity === "string",
      );
      assert(
        !active(j.supervisor),
        "Refusing recovery while the original supervisor is alive",
      );
      assert(j.files && typeof j.files === "object" && !Array.isArray(j.files));
      for (const [file, hash] of Object.entries(j.files)) {
        assert(
          ["compose.yaml", "tls.yaml", "deployment.env", "nginx.conf"].includes(
            file,
          ),
        );
        assert.equal(
          digest(join(directory, file)),
          hash,
          "Owned Compose inputs changed; reconcile before recovery",
        );
      }
      assert(Array.isArray(j.children));
      for (const c of j.children) {
        assert(
          typeof c.id === "string" &&
            typeof c.binary === "string" &&
            Array.isArray(c.args) &&
            c.args.every((a) => typeof a === "string"),
        );
        assert(
          c.phase === undefined ||
            ["intent", "spawned", "exited", "not_spawned"].includes(c.phase),
        );
        assert(
          c.owner === null ||
            (Number.isSafeInteger(c.owner.pid) &&
              typeof c.owner.identity === "string"),
        );
      }
      assert(
        j.cleanup &&
          typeof j.cleanup === "object" &&
          Array.isArray(j.cleanupHistory),
      );
      assert(
        j.details === undefined ||
          (j.details !== null &&
            typeof j.details === "object" &&
            !Array.isArray(j.details) &&
            Object.values(j.details).every(
              (value) => typeof value === "string",
            )),
      );
      return new Rehearsal(j, heldLock);
    } catch (error) {
      heldLock.close();
      throw error;
    }
  }
  get directory() {
    return this.journal.directory;
  }
  get signal() {
    return this.abort.signal;
  }
  save() {
    this.lock.assertHeld();
    atomic(join(this.directory, "ownership.json"), this.journal);
  }
  seal(files: string[]) {
    for (const name of files)
      this.journal.files[name] = digest(join(this.directory, name));
    this.save(); // before any container or native child effect
  }
  supervise() {
    for (const signal of ["SIGINT", "SIGTERM"] as const) {
      const handler = () => {
        if (!this.journal.interrupted) {
          this.journal.interrupted = signal;
          this.save();
          this.abort.abort(new Error(`Rehearsal interrupted by ${signal}`));
        }
      };
      this.handlers.push([signal, handler]);
      process.on(signal, handler);
    }
  }
  async wait(ms: number) {
    await pause(ms, undefined, { signal: this.signal });
  }
  child(binary: string, args: string[], cancellable = true) {
    if (cancellable) this.signal.throwIfAborted();
    const entry: OwnedChild = {
      id: randomUUID(),
      binary: resolve(binary),
      args,
      owner: null,
    };
    entry.phase = "intent";
    this.journal.children.push(entry);
    this.save();
    const p = spawn(binary, args, { stdio: ["pipe", "pipe", this.log] });
    // Keep the live handle before *any* fallible post-spawn lookup/write.
    this.handles.set(entry.id, p);
    p.on("error", () => {
      if (!p.pid) entry.phase = "not_spawned";
    });
    try {
      if (p.pid) {
        entry.phase = "spawned";
        entry.owner = { pid: p.pid, identity: "" };
        this.save();
        entry.owner.identity = identity(p.pid);
        this.save();
      }
    } catch (error) {
      // This known, just-spawned handle cannot be lost behind a lookup exception.
      // The caller's cleanup awaits exit and escalates if needed.
      p.kill("SIGINT");
      throw error;
    }
    return p;
  }
  async command(
    binary: string,
    args: string[],
    options: { input?: string; cleanup?: boolean; timeout?: number } = {},
  ) {
    const p = this.child(binary, args, !options.cleanup);
    const signal = options.cleanup ? undefined : this.signal;
    const chunks: Buffer[] = [];
    let size = 0;
    let failed = false;
    let escalation: ReturnType<typeof setTimeout> | undefined;
    const cancel = () => {
      failed = true;
      p.kill("SIGTERM");
      escalation ??= setTimeout(() => p.kill("SIGKILL"), 5000);
    };
    signal?.addEventListener("abort", cancel, { once: true });
    // Independent hard deadline also bounds a child that ignores TERM.
    const timer = setTimeout(() => {
      failed = true;
      p.kill("SIGKILL");
      p.stdout?.destroy();
      p.stdin?.destroy();
    }, options.timeout ?? 180_000);
    p.stdout?.on("data", (b: Buffer) => {
      size += b.length;
      if (size <= 8 * 1024 * 1024) chunks.push(b);
      else {
        failed = true;
        p.kill("SIGKILL");
      }
    });
    p.stdin?.on("error", () => {});
    p.stdin?.end(options.input);
    try {
      const code = await new Promise<number | null>((done, fail) => {
        p.once("error", fail);
        p.once("close", done);
      });
      assert(
        !failed && code === 0,
        "Owned command failed; inspect private.log and ownership.json",
      );
      return Buffer.concat(chunks);
    } finally {
      clearTimeout(timer);
      clearTimeout(escalation);
      signal?.removeEventListener("abort", cancel);
      if (p.exitCode !== null || p.signalCode !== null || !p.pid) {
        const entry = this.journal.children.find(
          (c) => this.handles.get(c.id) === p,
        );
        assert(entry);
        entry.phase = p.pid ? "exited" : "not_spawned";
        this.save();
      }
    }
  }
  compose(
    args: string[],
    options: { input?: string; cleanup?: boolean; timeout?: number } = {},
  ) {
    for (const name of ["compose.yaml", "tls.yaml", "deployment.env"]) {
      assert.equal(
        digest(join(this.directory, name)),
        this.journal.files[name],
        "Owned Compose inputs changed",
      );
    }
    return this.command(
      "docker",
      [
        "compose",
        "--env-file",
        join(this.directory, "deployment.env"),
        "-p",
        this.journal.project,
        "-f",
        join(this.directory, "compose.yaml"),
        "-f",
        join(this.directory, "tls.yaml"),
        ...args,
      ],
      options,
    );
  }
  async stop(p: ChildProcess) {
    const entry = this.journal.children.find(
      (c) => this.handles.get(c.id) === p,
    );
    assert(entry);
    await this.stopOwned(entry);
  }
  private async stopOwned(c: OwnedChild) {
    this.lock.assertHeld();
    const p = this.handles.get(c.id);
    const exited = () => {
      c.phase = "exited";
    };
    if (p && (p.exitCode !== null || p.signalCode !== null)) {
      exited();
      return;
    }
    if (c.phase === "exited" || c.phase === "not_spawned") return;
    if (p?.pid) {
      // This process still owns the ChildProcess handle, even if registration failed.
      for (const [signal, timeout] of [
        ["SIGINT", 5000],
        ["SIGKILL", 3000],
      ] as const) {
        if (p.exitCode !== null || p.signalCode !== null) {
          exited();
          return;
        }
        p.kill(signal);
        const end = Date.now() + timeout;
        while (Date.now() < end) {
          if (p.exitCode !== null || p.signalCode !== null) {
            exited();
            return;
          }
          await pause(100);
        }
      }
      throw new Error(
        "Known owned child did not stop; retain recovery directory",
      );
    }
    // A missing intent/PID/birth identity is uncertainty, never evidence of absence.
    assert(c.owner, "Unresolved spawn intent; retain exact recovery evidence");
    const original = c.owner;
    const sameOrGone = () => {
      const current = identity(original.pid);
      if (!current) return false;
      assert(
        original.identity && current === original.identity,
        "Child identity uncertain/changed; refuse unrelated signaling",
      );
      return true;
    };
    if (!sameOrGone()) {
      exited();
      return;
    }
    for (const [signal, timeout] of [
      ["SIGINT", 5000],
      ["SIGKILL", 3000],
    ] as const) {
      if (!sameOrGone()) {
        exited();
        return;
      }
      process.kill(original.pid, signal);
      const end = Date.now() + timeout;
      while (Date.now() < end) {
        if (!sameOrGone()) {
          exited();
          return;
        }
        await pause(100);
      }
    }
    throw new Error("Owned child did not stop; retain recovery directory");
  }
  cleanup() {
    return (this.cleaning ??= this.performCleanup());
  }
  private async performCleanup() {
    const j = this.journal;
    j.status = "cleaning";
    this.save();
    const attempt = async (name: string, fn: () => Promise<unknown>) => {
      try {
        await fn();
        j.cleanup[name] = { ok: true };
      } catch (error) {
        // Fixed errors describe a step; command output and credential values stay private.
        try {
          writeFileSync(
            this.log,
            `Cleanup ${name} failed (${error instanceof Error ? error.name : "unknown error"})\n`,
          );
        } catch {
          /* Diagnostic output must not prevent the remaining cleanup steps. */
        }
        j.cleanup[name] = {
          ok: false,
          error:
            "Step failed; retained private.log and exact ownership for retry",
        };
      }
      this.save();
    };
    // Cleanup commands can themselves fail after spawn. Drain each newly owned
    // handle before native logout and after teardown, with one shutdown bound per child.
    const checked = new Set<string>();
    let childFailure = false;
    const drainChildren = async () => {
      for (const c of [...j.children].reverse()) {
        if (checked.has(c.id)) continue;
        checked.add(c.id);
        try {
          await this.stopOwned(c);
        } catch {
          childFailure = true;
        }
      }
      assert(!childFailure, "Some owned children remain unresolved");
    };
    await attempt("children", drainChildren);
    await attempt("diagnostics", async () => {
      const diagnostics = await this.compose(["logs", "--no-color"], {
        cleanup: true,
        timeout: 15_000,
      });
      writeFileSync(this.log, diagnostics);
    });
    await attempt("children", drainChildren);
    await attempt("native", async () => {
      assert.equal(digest(j.cli), j.cliHash, "Creating CLI changed");
      if (!j.cleanup.children?.ok)
        throw new Error("Stop owned writers before native logout");
      const identityPath = join(j.state, "identity.json");
      // Login may have been interrupted mid-publication: invoke logout even if no identity file yet.
      await this.command(
        j.cli,
        ["logout", "--allow-fixture-http", "--fixture-state-path", j.state],
        { cleanup: true, timeout: 30_000 },
      );
      assert(
        existsSync(identityPath),
        "Logout must leave verifiable cleanup metadata",
      );
      // Select the cleanup protocol from the hash-verified creating binary;
      // a restored or malformed ordinary file cannot downgrade verification.
      const help = await this.command(j.cli, ["--help"], {
        cleanup: true,
        timeout: 10_000,
      });
      if (/^\s+upgrade-handoff\s/m.test(help.toString("utf8"))) {
        await this.command(
          j.cli,
          ["fixture-retire", "--fixture-state-path", j.state],
          { cleanup: true, timeout: 30_000 },
        );
        assert.equal(
          JSON.parse(readFileSync(identityPath, "utf8")).fixtureRetired,
          true,
        );
      }
      const metadata = JSON.parse(readFileSync(identityPath, "utf8")) as Record<
        string,
        unknown
      >;
      assert.equal(metadata.active, null);
      assert.deepEqual(metadata.credentialAccounts, []);
      assert.deepEqual(metadata.pendingCleanup, []);
    });
    await attempt("stack", async () => {
      // Keep central state available for native recovery when logout is incomplete.
      assert(
        j.cleanup.native?.ok,
        "Native cleanup incomplete; preserve stack for recovery",
      );
      await this.compose(
        ["down", "--volumes", "--remove-orphans", "--timeout", "15"],
        { cleanup: true, timeout: 60_000 },
      );
      const name = `${j.project}-keygen`;
      const found = (
        await this.command(
          "docker",
          ["ps", "-aq", "--filter", `name=^/${name}$`],
          { cleanup: true, timeout: 10_000 },
        )
      )
        .toString()
        .trim();
      if (found) {
        const label = (
          await this.command(
            "docker",
            [
              "inspect",
              "--format",
              '{{index .Config.Labels "dhivon.rehearsal"}}',
              name,
            ],
            { cleanup: true, timeout: 10_000 },
          )
        )
          .toString()
          .trim();
        assert.equal(label, j.project);
        await this.command("docker", ["rm", "-f", name], {
          cleanup: true,
          timeout: 15_000,
        });
      }
      for (const kind of ["container", "network", "volume"]) {
        const args = kind === "container" ? ["ps", "-aq"] : [kind, "ls", "-q"];
        assert.equal(
          (
            await this.command(
              "docker",
              [
                ...args,
                "--filter",
                `label=com.docker.compose.project=${j.project}`,
              ],
              { cleanup: true, timeout: 10_000 },
            )
          )
            .toString()
            .trim(),
          "",
          `Owned ${kind} remains`,
        );
      }
    });
    await attempt("children", drainChildren);
    const clean = ["children", "native", "stack"].every(
      (k) => j.cleanup[k]?.ok,
    );
    j.status = clean ? "clean" : "recovery_required";
    j.cleanupHistory.push({
      at: new Date().toISOString(),
      status: j.status,
      steps: structuredClone(j.cleanup),
    });
    this.save();
    atomic(join(this.directory, "result.json"), {
      ...j.details,
      succeeded:
        j.workflowSucceeded &&
        !j.interrupted &&
        clean &&
        j.cleanupHistory.every(
          (attempt) =>
            attempt.status === "clean" &&
            Object.values(attempt.steps).every((step) => step.ok),
        ),
      workflowSucceeded: j.workflowSucceeded,
      interrupted: j.interrupted,
      cleanupStatus: j.status,
      cleanup: j.cleanup,
      cleanupHistory: j.cleanupHistory,
      project: j.project,
      nativeCredentialsCleared: j.cleanup.native?.ok === true,
      stacksRemoved: j.cleanup.stack?.ok === true,
      certificateTrust:
        "Separate operator-owned action; this harness never changes OS/browser trust",
    });
    // Keep the kernel lock until the entry point finishes all evidence writes and closes.
    return clean;
  }
}
