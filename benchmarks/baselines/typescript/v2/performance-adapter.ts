/** Persistent M8 performance adapter for the frozen TypeScript V2 boundary. */

import { createInterface } from "node:readline";
import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { runCase, type JsonObject } from "./fixture.js";

type Command =
  | Readonly<{ command: "verify" }>
  | Readonly<{ command: "warmup"; iterations: number }>
  | Readonly<{
      command: "measure";
      duration_ns: number;
      sample_stride: number;
    }>
  | Readonly<{ command: "shutdown" }>;

function emit(value: unknown): void {
  process.stdout.write(`${JSON.stringify(value)}\n`);
}

function argument(name: string): string {
  const index = process.argv.indexOf(name);
  const value = process.argv[index + 1];
  if (index < 0 || value === undefined) {
    throw new Error(`missing ${name}`);
  }
  return value;
}

function parseJson(text: string): unknown {
  return JSON.parse(text) as unknown;
}

async function loadCases(manifestPath: string): Promise<unknown[]> {
  const manifest = parseJson(await readFile(manifestPath, "utf8")) as {
    fixtures: readonly Readonly<{ path: string }>[];
  };
  const root = resolve(dirname(manifestPath), "../..");
  return Promise.all(
    manifest.fixtures.map(async (entry) =>
      parseJson(await readFile(resolve(root, entry.path), "utf8")),
    ),
  );
}

function run(cases: readonly unknown[]): JsonObject[] {
  return cases.map((fixture) => runCase(fixture));
}

async function main(): Promise<void> {
  const cases = await loadCases(resolve(argument("--manifest")));
  emit({ type: "ready", case_count: cases.length });
  const lines = createInterface({ input: process.stdin, crlfDelay: Infinity });
  for await (const line of lines) {
    const command = JSON.parse(line) as Command;
    switch (command.command) {
      case "verify":
        emit({ type: "verified", results: run(cases) });
        break;
      case "warmup": {
        let checksum = 0;
        for (
          let iteration = 0;
          iteration < command.iterations;
          iteration += 1
        ) {
          checksum ^= run(cases).reduce(
            (total, result) => total + String(result.case_id).length,
            0,
          );
        }
        emit({
          type: "warmed",
          iterations: command.iterations,
          request_count: command.iterations * cases.length,
          checksum,
        });
        break;
      }
      case "measure": {
        const started = process.hrtime.bigint();
        const samples: number[] = [];
        let requestCount = 0;
        let checksum = 0;
        while (
          requestCount === 0 ||
          process.hrtime.bigint() - started < BigInt(command.duration_ns)
        ) {
          for (const fixture of cases) {
            const before = process.hrtime.bigint();
            const result = runCase(fixture);
            const elapsed = process.hrtime.bigint() - before;
            if (requestCount % command.sample_stride === 0) {
              samples.push(Number(elapsed));
            }
            requestCount += 1;
            checksum ^= String(result.case_id).length;
          }
        }
        emit({
          type: "measured",
          clock: "process.hrtime.bigint",
          elapsed_ns: Number(process.hrtime.bigint() - started),
          request_count: requestCount,
          sample_stride: command.sample_stride,
          samples_ns: samples,
          checksum,
        });
        break;
      }
      case "shutdown":
        emit({ type: "stopped" });
        lines.close();
        return;
      default:
        command satisfies never;
    }
  }
}

await main();
