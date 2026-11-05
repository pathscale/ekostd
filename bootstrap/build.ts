import { createHash } from "node:crypto";
import { mkdir, readFile, readdir, realpath, writeFile } from "node:fs/promises";
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
};
const repo = await realpath(fileURLToPath(new URL("../", import.meta.url)));
const options = new Map<string, string>();
let execute = false;
for (let i = 2; i < process.argv.length; i++) {
  const name = process.argv[i];
  if (name === "--run") { execute = true; continue; }
  if (!["--out", "--rustc", "--ekopathrs", "--ekors", "--socket"].includes(name)
      || options.has(name) || !process.argv[i + 1] || process.argv[i + 1].startsWith("--")) {
    throw new Error(`invalid or duplicate option: ${name}`);
  }
  options.set(name, process.argv[++i]);
}
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
const report = async () => writeFile(join(out, "outcomes.json"), JSON.stringify({
  mode: execute ? "run" : "plan", target: m.target, cfg: m.cfg, panic: m.panic,
  cwd: repo, environment: { RUSTC_BOOTSTRAP: "1", inherited: false },
  bootstrapAccepted: false, attempts,
}, null, 2) + "\n");
const rustc = options.get("--rustc") ?? "rustc";
const ekors = options.get("--ekors");
const socket = options.get("--socket");
async function run(a: Attempt): Promise<void> {
  attempts.push(a);
  await report();
  console.log(JSON.stringify({ stage: a.stage, unit: a.unit, argv: a.argv, status: a.status }));
  if (a.status !== "ready") return;
  a.stdout = join(out, `${a.stage}-${a.unit}.stdout`);
  a.stderr = join(out, `${a.stage}-${a.unit}.stderr`);
  try {
    const child = Bun.spawn(a.argv, {
      cwd: repo, env: { PATH: process.env.PATH ?? "/usr/bin:/bin", RUSTC_BOOTSTRAP: "1" },
      stdin: "ignore", stdout: Bun.file(a.stdout), stderr: Bun.file(a.stderr),
    });
    a.exitCode = await child.exited;
    a.status = a.exitCode === 0 ? "command-exited-zero" : "failed";
  } catch (e) { a.status = "spawn-failed"; a.reason = String(e); }
  await report();
}
const artifact = (u: Unit) => join(stage0, `lib${u.crate}.rlib`);
for (const u of m.units) {
  const argv = [rustc, roots.get(u.key)!, "--crate-name", u.crate, "--crate-type", "rlib",
    "--edition", u.edition, "--target", m.target, "-Cpanic=abort", "-Cdebug-assertions=no",
    "-Coverflow-checks=no", "--sysroot", sysroot, "-L", `dependency=${stage0}`,
    ...m.cfg.flatMap(c => ["--cfg", c]),
    ...u.dependencies.flatMap(d => ["--extern", `${units.get(d)!.crate}=${artifact(units.get(d)!)}`]),
    "-o", artifact(u)];
  const blocked = u.dependencies.filter(d => !attempts.some(a => a.stage === "stage0" && a.unit === d && a.status === "rlib-produced"));
  const a: Attempt = { stage: "stage0", unit: u.key, argv,
    status: !execute ? "not-run" : missing.has(u.key) ? "missing-source" : blocked.length ? "blocked" : "ready",
    reason: missing.get(u.key) ?? (blocked.length ? `dependencies: ${blocked.join(",")}` : undefined) };
  await run(a);
  if (a.status === "command-exited-zero") {
    a.status = await Bun.file(artifact(u)).exists() && (await Bun.file(artifact(u)).size) > 0 ? "rlib-produced" : "missing-artifact";
  }
  await report();
}
function closure(u: Unit, result = new Set<string>()): Set<string> {
  for (const d of u.dependencies) { if (!result.has(d)) { result.add(d); closure(units.get(d)!, result); } }
  result.add(u.key);
  return result;
}
for (const root of m.units.filter(u => !u.language)) {
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
  const objects = join(out, `stage1-${root.key}`);
  await mkdir(objects);
  const absent = selected.filter(u => missing.has(u.key)).map(u => u.key);
  const a: Attempt = { stage: "stage1", unit: root.key,
    argv: ekors && socket ? [ekors, "--socket", socket, "compile",
      `--source-graph=${graphPath}`, "--target", `dir=${objects}`] : [],
    status: !execute ? "not-run" : absent.length ? "missing-source" : !ekors || !socket ? "missing-eko-endpoint" : "ready",
    reason: absent.length ? `sources: ${absent.join(",")}` : undefined };
  await run(a);
  if (a.status === "command-exited-zero") {
    try {
      const emitted = (await readdir(objects, { recursive: true })).filter(n => n.endsWith(".o"));
      const nonempty = await Promise.all(emitted.map(n => Bun.file(join(objects, n)).size > 0));
      a.status = emitted.length > 0 && nonempty.every(Boolean) ? "source-objects-produced" : "request-or-object-failed";
    } catch (e) { a.status = "invalid-response"; a.reason = String(e); }
  }
  await report();
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
