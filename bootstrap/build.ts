import { createHash } from "node:crypto";
import { access, mkdir, readFile, readdir, realpath, unlink, writeFile } from "node:fs/promises";
import { dirname, isAbsolute, join, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

type Unit = {
  key: string; repository: "ekostd" | "ekopathrs"; root: string;
  package: string; version: string; crate: string; edition: string;
  language: boolean; dependencies: string[];
};
type Manifest = {
  schema: string; target: string; panic: string; cfg: string[]; units: Unit[];
};
type Attempt = {
  stage: string; unit: string; argv: string[]; status: string;
  exitCode?: number; reason?: string; stdout?: string; stderr?: string;
  elapsedMs: number; timedOut?: boolean;
  pass?: "initial" | "warmup" | "warm";
  cacheState?: "cold" | "shared-warm" | "external-unknown";
};
const repo = await realpath(fileURLToPath(new URL("../", import.meta.url)));
const options = new Map<string, string>();
let execute = false;
for (let i = 2; i < process.argv.length; i++) {
  const name = process.argv[i];
  if (name === "--run") { execute = true; continue; }
  if (["--per-unit-daemon", "--shared-socket", "--warmup"].includes(name)) {
    if (options.has(name)) throw new Error(`invalid or duplicate option: ${name}`);
    options.set(name, "true");
    continue;
  }
  if (!["--out", "--rustc", "--ekopathrs", "--ekors", "--socket", "--ekod-bin", "--timeout-ms"].includes(name)
      || options.has(name) || !process.argv[i + 1] || process.argv[i + 1].startsWith("--")) {
    throw new Error(`invalid or duplicate option: ${name}`);
  }
  options.set(name, process.argv[++i]);
}
const timeoutMs = Number(options.get("--timeout-ms") ?? "15000");
if (!Number.isSafeInteger(timeoutMs) || timeoutMs <= 0) throw new Error("--timeout-ms must be a positive integer");
const outOption = options.get("--out");
if (!outOption || !isAbsolute(outOption)) throw new Error("--out requires a fresh absolute directory");
const out = resolve(outOption);
if (out === repo || out.startsWith(repo + sep)) throw new Error("--out must be outside the source repository");
const compilerRepo = await realpath(options.get("--ekopathrs") ?? resolve(repo, "../ekopathrs"));
if (out === compilerRepo || out.startsWith(compilerRepo + sep)) throw new Error("--out must be outside ekopathrs");
const manifestPath = join(repo, "bootstrap/sources.json");
const manifestBytes = await readFile(manifestPath);
const m: Manifest = JSON.parse(manifestBytes.toString("utf8"));
if (m.schema !== "ekostd.bootstrap/1" || m.panic !== "abort"
    || typeof m.target !== "string" || !m.target || !Array.isArray(m.cfg)
    || m.cfg.some(c => typeof c !== "string") || !Array.isArray(m.units)) {
  throw new Error("invalid bootstrap configuration");
}
const units = new Map<string, Unit>();
for (const u of m.units) {
  if (!u || typeof u.key !== "string" || !/^[a-z_][a-z0-9_]*$/.test(u.key)
      || units.has(u.key) || !["ekostd", "ekopathrs"].includes(u.repository)
      || typeof u.root !== "string" || isAbsolute(u.root) || u.root.split(/[\\/]/).includes("..")
      || !u.root.endsWith("/lib.rs") || !/^[a-z_][a-z0-9_]*$/.test(u.crate)
      || typeof u.package !== "string" || typeof u.version !== "string"
      || !["2021", "2024"].includes(u.edition) || typeof u.language !== "boolean"
      || !Array.isArray(u.dependencies) || new Set(u.dependencies).size !== u.dependencies.length
      || u.dependencies.some(d => !units.has(d))) throw new Error(`invalid dependency-first unit: ${u?.key}`);
  units.set(u.key, u);
}
for (const [key, root] of [["core", "bootstrap/core/src/lib.rs"], ["alloc", "bootstrap/alloc/src/lib.rs"]]) {
  const u = units.get(key);
  if (!u || !u.language || u.crate !== key || u.repository !== "ekostd" || u.root !== root) {
    throw new Error(`candidate ${key} must use ${root}`);
  }
}
const builtins = units.get("compiler_builtins");
if (!builtins || builtins.language || builtins.repository !== "ekostd"
    || builtins.crate !== "compiler_builtins"
    || builtins.root !== "bootstrap/compiler_builtins/src/lib.rs"
    || m.units[0]?.key !== "core" || m.units[1]?.key !== "compiler_builtins"
    || JSON.stringify(builtins.dependencies) !== '["core"]') {
  throw new Error("compiler_builtins must follow core and use bootstrap/compiler_builtins/src/lib.rs");
}
if (m.units.some(u => u.language && u.key !== "core" && u.key !== "alloc")
    || units.get("core")!.dependencies.length !== 0
    || JSON.stringify(units.get("alloc")!.dependencies) !== '["core","compiler_builtins"]'
    || m.units.some(u => u.key !== "core" && u.key !== "compiler_builtins"
      && !u.dependencies.includes("compiler_builtins"))
    || m.units.filter(u => !u.language && u.key !== "compiler_builtins").length < 2) {
  throw new Error("invalid language roots, runtime dependencies or missing consumers");
}
await mkdir(out, { recursive: false });
const stage0 = join(out, "stage0");
const sysroot = join(out, "empty-sysroot");
await mkdir(stage0);
await mkdir(sysroot);
await writeFile(join(out, "sources.json"), manifestBytes);
const roots = new Map<string, string>();
const missing = new Map<string, string>();
for (const u of m.units) {
  const owner = u.repository === "ekostd" ? repo : compilerRepo;
  const path = resolve(owner, u.root);
  try {
    const canonical = await realpath(path);
    if (canonical !== path) throw new Error("source aliases are not admitted");
    roots.set(u.key, canonical);
  } catch (e) { roots.set(u.key, path); missing.set(u.key, String(e)); }
}
function hash(bytes: Uint8Array): string { return createHash("sha256").update(bytes).digest("hex"); }
async function snapshot(): Promise<Record<string, string>> {
  const files: Record<string, string> = {};
  async function walk(path: string): Promise<void> {
    for (const entry of (await readdir(path, { withFileTypes: true })).sort((a, b) => a.name.localeCompare(b.name))) {
      const child = join(path, entry.name);
      if (entry.isSymbolicLink()) throw new Error(`source symlink: ${child}`);
      if (entry.isDirectory()) await walk(child);
      else if (entry.isFile()) files[child] = hash(await readFile(child));
      else throw new Error(`non-file source input: ${child}`);
    }
  }
  files[manifestPath] = hash(await readFile(manifestPath));
  files[fileURLToPath(import.meta.url)] = hash(await readFile(fileURLToPath(import.meta.url)));
  for (const u of m.units) {
    if (missing.has(u.key)) files[roots.get(u.key)!] = `missing: ${missing.get(u.key)}`;
    else await walk(dirname(roots.get(u.key)!));
  }
  return Object.fromEntries(Object.entries(files).sort(([a], [b]) => a.localeCompare(b)));
}
const before = await snapshot();
await writeFile(join(out, "source-identities.json"), JSON.stringify(before, null, 2) + "\n");
const attempts: Attempt[] = [];
let reportQueue = Promise.resolve();
const report = () => {
  const benchmarks = attempts.filter(a => a.stage === "stage1").map(a => {
    const baseline = attempts.find(b => b.stage === "stage0" && b.unit === a.unit);
    return {
      unit: a.unit, pass: a.pass, cacheState: a.cacheState,
      measured: a.pass !== "warmup",
      stage0RustcMs: baseline?.elapsedMs ?? null, stage1EkodMs: a.elapsedMs,
      stage0Status: baseline?.status ?? null, stage1Status: a.status,
      speedupRatio: a.pass !== "warmup" && baseline?.status === "rlib-produced"
        && a.status === "source-objects-produced" && a.elapsedMs > 0
        ? baseline.elapsedMs / a.elapsedMs : null,
    };
  });
  const body = JSON.stringify({
    mode: execute ? "run" : "plan", target: m.target, cfg: m.cfg, panic: m.panic,
    cwd: repo, environment: { RUSTC_BOOTSTRAP: "1", inherited: false },
    bootstrapAccepted: false, attempts, benchmarks,
  }, null, 2) + "\n";
  reportQueue = reportQueue.then(() => writeFile(join(out, "outcomes.json"), body));
  return reportQueue;
};
const rustc = options.get("--rustc") ?? "rustc";
const ekors = options.get("--ekors");
const socket = options.get("--socket");
const defaultEkod = join(compilerRepo, "ekod/target/release/ekod");
const defaultEkodExists = await access(defaultEkod).then(() => true, () => false);
const ekodBin = options.get("--ekod-bin") ?? (defaultEkodExists ? defaultEkod : undefined);
if (options.has("--per-unit-daemon") && options.has("--shared-socket")) {
  throw new Error("--per-unit-daemon conflicts with --shared-socket");
}
if (options.has("--warmup") && options.has("--per-unit-daemon")) {
  throw new Error("--warmup requires a daemon shared across roots");
}
const dedicatedDaemons = !options.has("--shared-socket") && Boolean(ekodBin);
const perUnitDaemon = options.has("--per-unit-daemon");
let sharedDaemon: ReturnType<typeof Bun.spawn> | undefined;
let sharedDaemonFailure: string | undefined;
let sharedRequests = 0;
const sharedSocket = `/tmp/ekod-bootstrap-${process.pid}.sock`;
async function run(a: Attempt): Promise<void> {
  attempts.push(a);
  await report();
  console.log(JSON.stringify({ stage: a.stage, unit: a.unit, argv: a.argv, status: a.status }));
  if (a.status !== "ready") return;
  const logKey = `${a.stage}-${a.pass ? `${a.pass}-` : ""}${a.unit}`;
  a.stdout = join(out, `${logKey}.stdout`);
  a.stderr = join(out, `${logKey}.stderr`);
  const started = performance.now();
  try {
    const child = Bun.spawn(a.argv, {
      cwd: repo, env: { PATH: process.env.PATH ?? "/usr/bin:/bin", RUSTC_BOOTSTRAP: "1" },
      stdin: "ignore", stdout: Bun.file(a.stdout), stderr: Bun.file(a.stderr),
    });
    let timer: ReturnType<typeof setTimeout> | undefined;
    const outcome = await Promise.race([
      child.exited.then(code => ({ code })),
      new Promise<{ timeout: true }>(resolveTimeout => {
        timer = setTimeout(() => { a.timedOut = true; child.kill("SIGKILL"); resolveTimeout({ timeout: true }); }, timeoutMs);
      }),
    ]);
    if (timer) clearTimeout(timer);
    if ("timeout" in outcome) a.status = "timed-out";
    else { a.exitCode = outcome.code; a.status = a.exitCode === 0 ? "command-exited-zero" : "failed"; }
  } catch (e) { a.status = "spawn-failed"; a.reason = String(e); }
  a.elapsedMs = performance.now() - started;
  await report();
}
async function waitForSocket(path: string): Promise<void> {
  const { stat } = await import("node:fs/promises");
  const deadline = performance.now() + 3000;
  while (performance.now() < deadline) {
    if (await stat(path).then(() => true, () => false)) return;
    await new Promise(resolveWait => setTimeout(resolveWait, 50));
  }
  throw new Error(`ekod socket did not appear: ${path}`);
}
const artifact = (u: Unit) => join(stage0, `lib${u.crate}.rlib`);
const stage0Promises = new Map<string, Promise<Attempt>>();
for (const u of m.units) {
  const promise = (async (): Promise<Attempt> => {
    const dependencies = await Promise.all(u.dependencies.map(d => stage0Promises.get(d)!));
    const argv = [rustc, roots.get(u.key)!, "--crate-name", u.crate, "--crate-type", "rlib",
      "--edition", u.edition, "--target", m.target, "-Cpanic=abort", "-Cdebug-assertions=no",
      "-Coverflow-checks=no", "--sysroot", sysroot, "-L", `dependency=${stage0}`,
      ...m.cfg.flatMap(c => ["--cfg", c]),
      ...u.dependencies.flatMap(d => ["--extern", `${units.get(d)!.crate}=${artifact(units.get(d)!)}`]),
      "-o", artifact(u)];
    const blocked = dependencies.filter(a => a.status !== "rlib-produced").map(a => a.unit);
    const a: Attempt = { stage: "stage0", unit: u.key, argv,
      status: !execute ? "not-run" : missing.has(u.key) ? "missing-source" : blocked.length ? "blocked" : "ready",
      reason: missing.get(u.key) ?? (blocked.length ? `dependencies: ${blocked.join(",")}` : undefined), elapsedMs: 0 };
    await run(a);
    if (a.status === "command-exited-zero") {
      a.status = await Bun.file(artifact(u)).exists() && (await Bun.file(artifact(u)).size) > 0 ? "rlib-produced" : "missing-artifact";
    }
    await report();
    return a;
  })();
  stage0Promises.set(u.key, promise);
}
  const stage0Done = Promise.all(stage0Promises.values());
function closure(u: Unit, result = new Set<string>()): Set<string> {
  for (const d of u.dependencies) { if (!result.has(d)) { result.add(d); closure(units.get(d)!, result); } }
  result.add(u.key);
  return result;
}
await stage0Done;
const stage1Roots = m.units.filter(u => !u.language);
const passes: Array<NonNullable<Attempt["pass"]>> = options.has("--warmup")
  ? ["warmup", "warm"] : ["initial"];
try {
for (const pass of passes) {
for (const root of stage1Roots) {
  const selected = m.units.filter(u => closure(root).has(u.key));
  const language = selected.filter(u => u.language);
  const graph = {
    schema: "eko.source-graph/1", root: root.key,
    units: selected.map(u => ({ key: u.key, identity: {
      package_namespace: u.repository,
      package_name: u.package, package_version: u.version, crate_name: u.crate,
      logical_target: u.crate, logical_root: u.root, kind: "library", role: "target",
      target_triple: m.target, edition: u.edition, panic_strategy: m.panic, features: [], cfg: m.cfg,
    }, crate_root: roots.get(u.key), generator: "none",
    semantics: { debug_assertions: false, overflow_checks: false, rustflags: [] } })),
    edges: selected.filter(u => !u.language).flatMap(u => u.dependencies.filter(d => !units.get(d)!.language)
      .map(d => ({ from: u.key, extern_name: units.get(d)!.crate, to: d, kind: "rust", role: "target" }))),
    language_roots: language.map(u => ({ crate: u.crate, crate_root: roots.get(u.key),
      edition: u.edition, panic_strategy: m.panic, target_triple: m.target, cfg: m.cfg })),
  };
  const graphPath = join(out, `${root.key}.source-graph.json`);
  await writeFile(graphPath, JSON.stringify(graph, null, 2) + "\n");
  const objects = join(out, `stage1-${pass}-${root.key}`);
  await mkdir(objects);
  const absent = selected.filter(u => missing.has(u.key)).map(u => u.key);
  const shortKey = root.key.slice(0, 8);
  const dedicatedSocket = dedicatedDaemons
    ? perUnitDaemon ? `/tmp/ekod-${shortKey}-${process.pid}.sock` : sharedSocket : socket;
  let daemon: ReturnType<typeof Bun.spawn> | undefined;
  const a: Attempt = { stage: "stage1", unit: root.key, pass,
    cacheState: !dedicatedDaemons ? "external-unknown"
      : perUnitDaemon || sharedRequests === 0 ? "cold" : "shared-warm",
    argv: ekors && dedicatedSocket ? [ekors, "--socket", dedicatedSocket, "compile", "--timing",
      `--source-graph=${graphPath}`, "--target", `dir=${objects}`] : [],
    status: !execute ? "not-run" : absent.length ? "missing-source" : !ekors || !dedicatedSocket ? "missing-eko-endpoint" : "ready",
    reason: absent.length ? `sources: ${absent.join(",")}` : undefined, elapsedMs: 0 };
  try {
    if (a.status === "ready" && sharedDaemonFailure && !perUnitDaemon) {
      a.status = "daemon-start-failed"; a.reason = sharedDaemonFailure;
    }
    if (a.status === "ready" && dedicatedDaemons && (perUnitDaemon || !sharedDaemon)) {
      await Promise.all([unlink(dedicatedSocket!).catch(() => {}), unlink(`${dedicatedSocket}.control`).catch(() => {})]);
      const daemonHome = join(out, perUnitDaemon ? `home-${root.key}` : "home-shared");
      await mkdir(daemonHome, { recursive: true });
      const daemonLog = join(out, perUnitDaemon ? `ekod-${root.key}.stderr` : "ekod-shared.stderr");
      // Keep relative diagnostic files, including ekod-stage-progress.tsv, in out.
      daemon = Bun.spawn([ekodBin!, "--socket", dedicatedSocket!], {
        cwd: out, env: { ...process.env, HOME: daemonHome,
          EKOD_SOURCE_MEMORY_TRACE: process.env.EKOD_SOURCE_MEMORY_TRACE ?? '1' },
        stdin: "ignore", stdout: Bun.file(daemonLog), stderr: Bun.file(daemonLog),
      });
      if (!perUnitDaemon) sharedDaemon = daemon;
      try { await waitForSocket(dedicatedSocket!); }
      catch (e) {
        daemon.kill("SIGKILL"); await daemon.exited;
        a.status = "daemon-start-failed";
        a.reason = `${String(e)} (log: ${await readFile(daemonLog, "utf8").catch(() => "")})`;
        if (!perUnitDaemon) sharedDaemonFailure = a.reason;
      }
    }
    if (a.status === "ready") sharedRequests++;
    await run(a);
    if (a.status === "command-exited-zero") {
      try {
        const emitted = (await readdir(objects, { recursive: true })).filter(n => n.endsWith(".o"));
        const nonempty = await Promise.all(emitted.map(n => Bun.file(join(objects, n)).size > 0));
        a.status = emitted.length > 0 && nonempty.every(Boolean) ? "source-objects-produced" : "request-or-object-failed";
      } catch (e) { a.status = "invalid-response"; a.reason = String(e); }
    }
    await report();
  } finally {
    if (daemon && perUnitDaemon) {
      daemon.kill("SIGKILL");
      await daemon.exited;
      // Archive progress before another isolated daemon can overwrite it.
      const progress = join(out, "ekod-stage-progress.tsv");
      const bytes = await readFile(progress).catch(() => undefined);
      if (bytes) await writeFile(join(out, `ekod-stage-progress-${pass}-${root.key}.tsv`), bytes);
      if (dedicatedSocket) await Promise.all([unlink(dedicatedSocket).catch(() => {}), unlink(`${dedicatedSocket}.control`).catch(() => {})]);
    }
  }
}
}
} finally {
  if (sharedDaemon) {
    sharedDaemon.kill("SIGKILL");
    await sharedDaemon.exited;
    await Promise.all([unlink(sharedSocket).catch(() => {}), unlink(`${sharedSocket}.control`).catch(() => {})]);
  }
}
try {
  const after = await snapshot();
  await writeFile(join(out, "source-identities-after.json"), JSON.stringify(after, null, 2) + "\n");
  attempts.push({ stage: "parity", unit: "all", argv: [],
    status: JSON.stringify(before) === JSON.stringify(after) ? "source-snapshots-identical" : "source-changed" });
} catch (e) { attempts.push({ stage: "parity", unit: "all", argv: [], status: "source-unavailable", reason: String(e) }); }
await report();
if (execute && attempts.some(a => !["rlib-produced", "source-objects-produced", "source-snapshots-identical"].includes(a.status))) {
  process.exitCode = 1;
}
