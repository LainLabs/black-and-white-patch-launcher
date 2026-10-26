import { existsSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";
import { FIELDS } from "./schema";

// The shipped settings: the payload copy (scripts/sync-payload.ps1), else the decomp checkout next to this repo.
const candidates = ["../../payload/files/BWDecomp.ini", "../../../BlackWhiteDecomp/BWDecomp.ini"].map((p) =>
  fileURLToPath(new URL(p, import.meta.url)),
);
const shipped = candidates.find((p) => existsSync(p));

function parse(text: string): { section: string; key: string; value: string }[] {
  const out: { section: string; key: string; value: string }[] = [];
  let section = "";
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trim();
    if (!line || line.startsWith(";")) continue;
    const s = /^\[(.+)\]$/.exec(line);
    if (s) section = s[1].trim();
    else if (line.includes("=")) {
      const i = line.indexOf("=");
      out.push({ section, key: line.slice(0, i).trim(), value: line.slice(i + 1).trim() });
    }
  }
  return out;
}

describe("settings schema", () => {
  it("has unique ids", () => {
    expect(new Set(FIELDS.map((f) => f.id)).size).toBe(FIELDS.length);
  });

  it("gives choices options that include the fallback", () => {
    for (const f of FIELDS.filter((f) => f.type === "choice")) {
      expect(f.options?.map((o) => o.value), f.id).toContain(f.fallback);
    }
  });

  it.skipIf(!shipped)("covers every key of the shipped BWDecomp.ini with its default", () => {
    const values = parse(readFileSync(shipped!, "utf8"));
    expect(values.length).toBeGreaterThan(20);
    for (const v of values) {
      const field = FIELDS.find((f) => f.store === "ini" && f.section === v.section && f.key === v.key);
      expect(field, `${v.section}.${v.key} is missing from schema.ts`).toBeDefined();
      expect(field!.fallback, `${v.section}.${v.key} default`).toBe(v.value);
    }
    // And nothing in the schema that the decomp no longer reads.
    for (const f of FIELDS.filter((f) => f.store === "ini")) {
      expect(values.some((v) => v.section === f.section && v.key === f.key), `${f.id} is not in BWDecomp.ini`).toBe(true);
    }
  });
});
