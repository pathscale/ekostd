import { mkdir, readFile, realpath, writeFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

type Dep = { name: string; pkg: string; dep_kinds: Array<{ kind: string | null }> };
type Node = { id: string; deps: Dep[]; features: string[] };
type Target = { kind: string[]; name: string; src_path: string; edition: string };
type Pkg = { id: string; name: string; version: string; source: string | null; edition: string; targets: Target[] };
type Metadata = { packages: Pkg[]; resolve: { nodes: Node[]; root: string | null } | null };
type Unit = {
  key: string; repository: string; root: string;
  package: string; version: string; crate: string; edition: string;
  language: boolean; dependencies: string[];
  features?: string[]; extern_names?: Record<string, string>; build_script?: string;
};
type Manifest = { schema: string; target: string; panic: string; cfg: string[]; units: Unit[] };

const repo = await realpath(fileURLToPath(new URL("../", import.meta.url)));
const options = new Map<string, string>();
for (let i = 2; i < process.argv.length; i++) {
  const name = process.argv[i];
  if (!["--manifest", "--out"].includes(name) || options.has(name)
      || !process.argv[i + 1] || process.argv[i + 1].startsWith("--")) {
    throw new Error(`invalid or duplicate option: ${name}`);
  }
  options.set(name, process.argv[++i]);
}
const manifestOption = options.get("--manifest");
const outOption = options.get("--out");
if (!manifestOption || !outOption) throw new Error("usage: units-from-cargo.ts --manifest PATH --out FILE");
const manifest = resolve(manifestOption);
const outPath = resolve(outOption);

const base: Manifest = JSON.parse(await readFile(join(repo, "bootstrap/sources.json"), "utf8"));
if (base.schema !== "ekostd.bootstrap/1") throw new Error("bootstrap/sources.json has an unknown schema");
const languageKeys = ["core", "compiler_builtins", "alloc"];
const language = languageKeys.map(key => {
  const u = base.units.find(x => x.key === key);
  if (!u) throw new Error(`bootstrap/sources.json has no unit ${key}`);
  return u;
});

const child = Bun.spawn(["cargo", "metadata", "--format-version", "1", "--offline",
  "--filter-platform", base.target, "--manifest-path", manifest], {
  cwd: dirname(manifest), env: { ...process.env, RUSTUP_TOOLCHAIN: "stable" },
  stdin: "ignore", stdout: "pipe", stderr: "pipe",
});
const [stdout, stderr, code] = await Promise.all([
  new Response(child.stdout).text(), new Response(child.stderr).text(), child.exited,
]);
if (code !== 0) throw new Error(`cargo metadata failed with exit code ${code}: ${stderr}`);
const meta: Metadata = JSON.parse(stdout);
if (!meta.resolve || !meta.resolve.root) throw new Error("the manifest must be a package, not a virtual workspace");

const packages = new Map(meta.packages.map(p => [p.id, p]));
const nodes = new Map(meta.resolve.nodes.map(n => [n.id, n]));
const pkg = (id: string): Pkg => packages.get(id)!;
const isProcMacro = (p: Pkg) => p.targets.some(t => t.kind.includes("proc-macro"));
const libTarget = (p: Pkg) => p.targets.find(t => t.kind.some(k => k === "lib" || k === "rlib" || k === "proc-macro"));
function libOf(p: Pkg): Target {
  const t = libTarget(p);
  if (!t) throw new Error(`package ${p.name} ${p.version} has no library target`);
  return t;
}
const crateOf = (p: Pkg) => libOf(p).name.replace(/-/g, "_");
const sorted = (ids: Iterable<string>) => [...new Set(ids)]
  .sort((a, b) => `${pkg(a).name} ${pkg(a).version}`.localeCompare(`${pkg(b).name} ${pkg(b).version}`));
// kind null is a normal dependency; "build" is a build dependency; "dev" is never followed.
const edges = (id: string, kind: "normal" | "build") => sorted(nodes.get(id)!.deps
  .filter(d => d.dep_kinds.some(k => k.kind === (kind === "normal" ? null : "build"))).map(d => d.pkg));

// Linked set: normal dependencies from the root, never entering a proc-macro package.
// The visit order is dependency first.
const linked: string[] = [];
const seen = new Set<string>();
function visit(id: string): void {
  if (seen.has(id)) return;
  seen.add(id);
  for (const d of edges(id, "normal")) if (!isProcMacro(pkg(d))) visit(d);
  linked.push(id);
}
visit(meta.resolve.root);

// Host set: what proc macros and build scripts need, including their own dependencies.
const host = new Map<string, string>();
const queue: string[] = [];
function addHost(id: string, direct: boolean): void {
  if (host.has(id)) return;
  host.set(id, isProcMacro(pkg(id)) ? "proc-macro" : direct ? "build-dependency" : "host-dependency");
  queue.push(id);
}
for (const id of linked) {
  for (const d of edges(id, "build")) addHost(d, true);
  for (const d of edges(id, "normal")) if (isProcMacro(pkg(d))) addHost(d, true);
}
for (let next = queue.shift(); next !== undefined; next = queue.shift()) {
  for (const d of [...edges(next, "normal"), ...edges(next, "build")]) addHost(d, false);
}

function assignKeys(ids: string[], reserved: Set<string>): Map<string, string> {
  const counts = new Map<string, number>();
  for (const id of ids) counts.set(crateOf(pkg(id)), (counts.get(crateOf(pkg(id))) ?? 0) + 1);
  const keys = new Map<string, string>();
  const taken = new Set(reserved);
  for (const id of ids) {
    const p = pkg(id);
    const crate = crateOf(p);
    const key = counts.get(crate)! > 1 ? `${crate}_${p.version.replace(/[^A-Za-z0-9]/g, "_")}` : crate;
    if (!/^[a-z_][a-z0-9_]*$/.test(key)) throw new Error(`unit key is not admitted by build.ts: ${key}`);
    if (taken.has(key)) throw new Error(`duplicate or reserved unit key: ${key} (${p.name} ${p.version})`);
    taken.add(key);
    keys.set(id, key);
  }
  return keys;
}
const buildScript = async (p: Pkg) => {
  const t = p.targets.find(x => x.kind.includes("custom-build"));
  return t ? await realpath(t.src_path) : undefined;
};

const linkedLibs = linked.filter(id => libTarget(pkg(id)));
const keys = assignKeys(linkedLibs, new Set(languageKeys));
const units: Unit[] = [...language];
for (const id of linkedLibs) {
  const p = pkg(id);
  const t = libOf(p);
  const normal = nodes.get(id)!.deps.filter(d => d.dep_kinds.some(k => k.kind === null) && keys.has(d.pkg));
  const externNames: Record<string, string> = {};
  for (const d of normal) {
    const key = keys.get(d.pkg)!;
    if (d.name !== crateOf(pkg(d.pkg)) && !(key in externNames)) externNames[key] = d.name;
  }
  const script = await buildScript(p);
  units.push({
    key: keys.get(id)!, repository: p.source === null ? "local" : "registry",
    root: await realpath(t.src_path), package: p.name, version: p.version, crate: crateOf(p),
    edition: t.edition ?? p.edition, language: false,
    dependencies: [...languageKeys, ...[...new Set(normal.map(d => keys.get(d.pkg)!))].sort()],
    features: [...nodes.get(id)!.features].sort(),
    ...(Object.keys(externNames).length ? { extern_names: externNames } : {}),
    ...(script ? { build_script: script } : {}),
  });
}

const hostIds = sorted(host.keys());
const hostKeys = assignKeys(hostIds, new Set());
const hostUnits = [];
for (const id of hostIds) {
  const p = pkg(id);
  const t = libOf(p);
  const script = await buildScript(p);
  hostUnits.push({
    key: hostKeys.get(id)!, kind: host.get(id)!, repository: p.source === null ? "local" : "registry",
    root: await realpath(t.src_path), package: p.name, version: p.version, crate: crateOf(p),
    edition: t.edition ?? p.edition,
    dependencies: [...new Set([...edges(id, "normal"), ...edges(id, "build")].filter(d => hostKeys.has(d))
      .map(d => hostKeys.get(d)!))].sort(),
    features: [...nodes.get(id)!.features].sort(),
    ...(script ? { build_script: script } : {}),
  });
}

await mkdir(dirname(outPath), { recursive: true });
await writeFile(outPath, JSON.stringify({
  schema: base.schema, target: base.target, panic: base.panic, cfg: base.cfg,
  units, host_units: hostUnits,
}, null, 2) + "\n");
console.log(`${outPath}: ${units.length - language.length} cargo units, ${hostUnits.length} host units`);
