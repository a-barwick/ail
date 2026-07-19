/** Language-neutral one-case and corpus runner. */

import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { dirname, isAbsolute, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

import { FixtureError, runCase, type JsonObject } from "./fixture.js";

export class RunnerError extends Error {}

export async function repositoryRoot(start = process.cwd()): Promise<string> {
  let current = resolve(start);
  for (;;) {
    try {
      const packageData = await readFile(join(current, ".git", "HEAD"));
      if (packageData.length > 0) {
        return current;
      }
    } catch {
      // Continue walking toward the filesystem root.
    }
    const parent = dirname(current);
    if (parent === current) {
      throw new RunnerError(
        "runner working directory is not inside the repository",
      );
    }
    current = parent;
  }
}

export async function run(arguments_: readonly string[]): Promise<JsonObject> {
  if (
    arguments_.length !== 2 ||
    (arguments_[0] !== "--case" && arguments_[0] !== "--corpus")
  ) {
    throw new RunnerError(
      "expected exactly --case <fixture> or --corpus <manifest>",
    );
  }
  const root = await repositoryRoot();
  const suppliedPath = arguments_[1]!;
  const path = isAbsolute(suppliedPath)
    ? suppliedPath
    : join(root, suppliedPath);
  return arguments_[0] === "--case" ? runCaseFile(path) : runCorpus(path, root);
}

async function loadJson(path: string): Promise<unknown> {
  try {
    return JSON.parse(await readFile(path, "utf8")) as unknown;
  } catch (error) {
    throw new RunnerError(`could not read or parse ${path}: ${String(error)}`, {
      cause: error,
    });
  }
}

async function runCaseFile(path: string): Promise<JsonObject> {
  try {
    return runCase(await loadJson(path));
  } catch (error) {
    if (error instanceof FixtureError) {
      throw new RunnerError(`could not run ${path}: ${error.message}`, {
        cause: error,
      });
    }
    throw error;
  }
}

async function runCorpus(path: string, root: string): Promise<JsonObject> {
  const bytes = await readFile(path);
  const manifest = await loadJson(path);
  if (
    typeof manifest !== "object" ||
    manifest === null ||
    !("fixtures" in manifest) ||
    !Array.isArray(manifest.fixtures)
  ) {
    throw new RunnerError(`could not parse ${path}: fixtures must be an array`);
  }
  const entries = manifest.fixtures as readonly unknown[];
  const results: JsonObject[] = [];
  for (const entry of entries) {
    if (
      typeof entry !== "object" ||
      entry === null ||
      !("path" in entry) ||
      typeof entry.path !== "string"
    ) {
      throw new RunnerError(
        "fixture manifest entry must contain a string path",
      );
    }
    results.push(await runCaseFile(join(root, entry.path)));
  }
  return {
    result_format: 1,
    fixture_manifest_sha256: createHash("sha256").update(bytes).digest("hex"),
    results,
  };
}

export async function runCli(arguments_: readonly string[]): Promise<number> {
  try {
    process.stdout.write(`${JSON.stringify(await run(arguments_))}\n`);
    return 0;
  } catch (error) {
    process.stderr.write(`typescript baseline runner: ${String(error)}\n`);
    return 1;
  }
}

if (
  process.argv[1] !== undefined &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  process.exitCode = await runCli(process.argv.slice(2));
}
