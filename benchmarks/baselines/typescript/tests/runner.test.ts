import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, it } from "node:test";

import { RunnerError, repositoryRoot as findRoot, run } from "../v2/runner.js";
import { repositoryRoot } from "./support.js";

const highPriorityFixture =
  "benchmarks/fixtures/public/create_job/" +
  "uc003-v2-created-priority-high.json";

describe("runner", () => {
  for (const arguments_ of [
    [],
    ["--case"],
    ["--case", "x", "y"],
    ["--unknown", "x"],
  ]) {
    it(`rejects argument shape ${JSON.stringify(arguments_)}`, async () => {
      await assert.rejects(() => run(arguments_), /expected exactly/u);
    });
  }

  it("accepts relative and absolute case paths", async () => {
    const relative = await run(["--case", highPriorityFixture]);
    const absolute = await run([
      "--case",
      join(repositoryRoot, highPriorityFixture),
    ]);
    assert.deepEqual(relative, absolute);
    assert.equal(relative.case_id, "uc003-v2-created-priority-high");
  });

  it("preserves corpus digest, count, and order", async () => {
    const result = await run(["--corpus", "benchmarks/fixtures/manifest.json"]);
    const manifest = await readFile(
      join(repositoryRoot, "benchmarks/fixtures/manifest.json"),
    );
    assert.equal(
      result.fixture_manifest_sha256,
      createHash("sha256").update(manifest).digest("hex"),
    );
    const cases = result.results as readonly Record<string, unknown>[];
    assert.equal(cases.length, 37);
    assert.equal(cases[0]?.case_id, "uc001-v1-created-empty-payload");
    assert.equal(cases.at(-1)?.case_id, "uc003-v1-stored-job-adapted");
  });

  it("rejects missing and malformed inputs", async () => {
    await assert.rejects(
      () => run(["--case", "benchmarks/fixtures/public/missing.json"]),
      /could not read/u,
    );
    const directory = await mkdtemp(join(tmpdir(), "ail-ts-runner-"));
    const malformed = join(directory, "manifest.json");
    await writeFile(malformed, "{");
    await assert.rejects(
      () => run(["--corpus", malformed]),
      /could not read or parse/u,
    );
  });

  for (const manifest of [
    {},
    { fixtures: [{}] },
    { fixtures: [{ path: 3 }] },
  ]) {
    it(`rejects malformed manifest ${JSON.stringify(manifest)}`, async () => {
      const directory = await mkdtemp(join(tmpdir(), "ail-ts-runner-"));
      const path = join(directory, "manifest.json");
      await writeFile(path, JSON.stringify(manifest));
      await assert.rejects(() => run(["--corpus", path]), RunnerError);
    });
  }

  it("fails repository discovery outside a checkout", async () => {
    await assert.rejects(() => findRoot(tmpdir()), /not inside/u);
  });
});
