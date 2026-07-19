import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { describe, it } from "node:test";

import { repositoryRoot } from "./support.js";

type Checkpoint = Readonly<{
  id: string;
  source_tree_sha256: string;
  files: readonly string[];
}>;

async function load(path: string): Promise<Record<string, unknown>> {
  return JSON.parse(await readFile(path, "utf8")) as Record<string, unknown>;
}

async function treeDigest(files: readonly string[]): Promise<string> {
  const records: string[] = [];
  for (const path of files) {
    const digest = createHash("sha256")
      .update(await readFile(join(repositoryRoot, path)))
      .digest("hex");
    records.push(`${digest}  ${path}\n`);
  }
  return createHash("sha256").update(records.join("")).digest("hex");
}

describe("frozen baseline contract", () => {
  it("matches both checkpoint source-tree digests", async () => {
    const manifest = await load(
      join(repositoryRoot, "benchmarks/baselines/typescript/checkpoints.json"),
    );
    const checkpoints = manifest.checkpoints as readonly Checkpoint[];
    assert.equal(checkpoints.length, 2);
    for (const checkpoint of checkpoints) {
      assert.equal(
        await treeDigest(checkpoint.files),
        checkpoint.source_tree_sha256,
      );
    }
  });

  it("covers every frozen hidden seed category once", async () => {
    const hidden = await load(
      join(repositoryRoot, "benchmarks/contracts/hidden-contract.json"),
    );
    const locations = await load(
      join(
        repositoryRoot,
        "benchmarks/baselines/typescript/seed-locations.json",
      ),
    );
    assert.equal(locations.language, "typescript");
    const categories = hidden.seed_categories as readonly {
      id: string;
    }[];
    const seeds = locations.locations as readonly {
      seed_id: string;
      semantic_locations: readonly string[];
    }[];
    assert.deepEqual(
      seeds.map((seed) => seed.seed_id),
      categories.map((category) => category.id),
    );
    assert.ok(seeds.every((seed) => seed.semantic_locations.length > 0));
  });
});
