import { mkdir } from "node:fs/promises";
import { resolve } from "node:path";

const label = "worker-runtime-witness";
const witness = "witnesses/worker-runtime.rs";
const barrier = "src/worker_runtime.rs::Barrier::wait";

function fail(message: string): never {
  console.error(`${label}: ${witness}: ${message}; barrier=${barrier}`);
  process.exit(1);
}

const args = Bun.argv.slice(2);
if (args.length !== 4) {
  fail(
    "usage: bun scripts/worker-runtime-witness.ts <stage1-rustc> <libeko.rlib> <dependency-directory> <output-directory>",
  );
}

const [compiler, runtime, dependencies, output] = args.map((argument) =>
  resolve(argument),
);
const allowedOutput = "/Users/revenge/code/chuzz/patches/grok-lanes";
if (!output.startsWith(`${allowedOutput}/`)) {
  fail(`output directory must be a child of ${allowedOutput}`);
}
const root = resolve(import.meta.dir, "..");
await mkdir(output, { recursive: true });

async function run(command: string[], timeoutMs: number) {
  const child = Bun.spawn(command, {
    cwd: root,
    stdin: "ignore",
    stdout: "pipe",
    stderr: "pipe",
  });
  let timedOut = false;
  const timeout = setTimeout(() => {
    timedOut = true;
    child.kill();
  }, timeoutMs);
  try {
    const [status, stdout, stderr] = await Promise.all([
      child.exited,
      new Response(child.stdout).text(),
      new Response(child.stderr).text(),
    ]);
    if (timedOut) {
      fail(`command timed out: ${command[0]}; inspect ${output}`);
    }
    return { status, stdout, stderr };
  } finally {
    clearTimeout(timeout);
  }
}

async function compile(source: string, name: string) {
  const executable = resolve(output, name);
  const result = await run(
    [
      compiler,
      "--edition=2021",
      "-Cpanic=abort",
      "-Copt-level=2",
      "-Cprefer-dynamic=no",
      "--extern",
      `eko=${runtime}`,
      "-L",
      `dependency=${dependencies}`,
      "--print=link-args",
      resolve(root, source),
      "-o",
      executable,
    ],
    120_000,
  );
  await Bun.write(resolve(output, `${name}.build.stdout`), result.stdout);
  await Bun.write(resolve(output, `${name}.build.stderr`), result.stderr);
  if (result.status !== 0) {
    fail(`compilation failed for ${source}; inspect ${name}.build.stderr`);
  }
  if (result.stdout.trim().length === 0) {
    fail(`compiler supplied no link arguments for ${source}`);
  }
  if (
    /(?:^|[/\s"'=])libstd(?:-[0-9a-f]+)?\.(?:rlib|so|dylib|a)\b/m.test(
      result.stdout,
    ) ||
    /(?:^|\s)-lstd(?:\s|$)/m.test(result.stdout)
  ) {
    fail(`Rust standard library is linked by ${source}`);
  }
  return executable;
}

const referenceExecutable = await compile(
  "witnesses/worker-runtime-serial.rs",
  "worker-runtime-serial",
);
const runtimeExecutable = await compile(witness, "worker-runtime");
const referenceResult = await run([referenceExecutable], 30_000);
const runtimeResult = await run([runtimeExecutable], 30_000);

await Promise.all([
  Bun.write(resolve(output, "reference.stdout"), referenceResult.stdout),
  Bun.write(resolve(output, "reference.stderr"), referenceResult.stderr),
  Bun.write(resolve(output, "runtime.stdout"), runtimeResult.stdout),
  Bun.write(resolve(output, "runtime.stderr"), runtimeResult.stderr),
]);

if (referenceResult.status !== 0) {
  fail("serial stage1 reference failed independently");
}
if (runtimeResult.stdout !== referenceResult.stdout) {
  fail("output disagreement with serial stage1 reference");
}
if (runtimeResult.status !== referenceResult.status) {
  fail(`runtime exit status ${runtimeResult.status} differs from serial stage1 reference`);
}
const identities = runtimeResult.stderr.match(
  /^identities caller=(\d+) workers=(\d+),(\d+),(\d+),(\d+)$/m,
);
if (!identities) {
  fail("runtime did not report OS worker identities");
}
const caller = identities[1];
const workers = identities.slice(2);
if (
  new Set(workers).size !== 4 ||
  workers.some((identity) => identity === "0" || identity === caller)
) {
  fail("runtime did not use four distinct OS workers");
}

console.log(
  `${label}: ${witness}: serial stage1 output matched; four distinct OS workers; final-worker barrier, cancellation, refusal, abort and shutdown verified; no Rust std linkage`,
);

