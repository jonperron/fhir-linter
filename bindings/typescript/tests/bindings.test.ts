import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, relative } from "node:path";
import { describe, expect, it } from "vitest";
import { lint, validate } from "../index.js";

const BARE_PATIENT = JSON.stringify({ resourceType: "Patient", id: "p1" });

describe("validate", () => {
  it("reports no errors for a valid resource", async () => {
    const diagnostics = await validate(BARE_PATIENT);
    expect(diagnostics.filter((d) => d.severity === "error")).toEqual([]);
  });

  it("reports an unknown resource type", async () => {
    const diagnostics = await validate('{"resourceType": "Nope"}');
    expect(diagnostics.map((d) => d.code)).toEqual(["STRUCTURE_000"]);
    expect(diagnostics[0]?.severity).toBe("error");
    expect(diagnostics[0]?.rule).toBeUndefined();
  });

  it("reports an unknown element", async () => {
    const diagnostics = await validate('{"resourceType": "Patient", "bogus": 1}');
    expect(diagnostics.some((d) => d.path === "Patient.bogus")).toBe(true);
  });
});

describe("lint", () => {
  it("triggers all builtin rules on a bare patient", async () => {
    const diagnostics = await lint(BARE_PATIENT);
    expect(diagnostics.map((d) => d.rule).sort()).toEqual([
      "missing-identifier",
      "missing-meta-profile",
      "narrative-absent",
    ]);
    expect(new Set(diagnostics.map((d) => d.severity))).toEqual(new Set(["warning"]));
  });

  it("applies config severity and enabled overrides", async () => {
    const config = `
[rules.narrative-absent]
severity = "error"

[rules.missing-identifier]
enabled = false
`;
    const byRule = new Map((await lint(BARE_PATIENT, config)).map((d) => [d.rule, d]));
    expect(byRule.has("missing-identifier")).toBe(false);
    expect(byRule.get("narrative-absent")?.severity).toBe("error");
  });

  it("rejects an unknown rule", async () => {
    await expect(lint(BARE_PATIENT, "[rules.bogus]\nenabled = false")).rejects.toThrow(
      /unknown lint rule/,
    );
  });

  it("rejects an unknown severity", async () => {
    await expect(
      lint(BARE_PATIENT, '[rules.narrative-absent]\nseverity = "loud"'),
    ).rejects.toThrow();
  });

  it("rejects invalid TOML", async () => {
    await expect(lint(BARE_PATIENT, "[rules")).rejects.toThrow(/invalid lint config/);
  });
});

describe("errors", () => {
  it.each([["validate", validate], ["lint", lint]] as const)("%s rejects malformed JSON", async (_name, fn) => {
    await expect(fn("{not json")).rejects.toThrow(/parse error/);
  });

  it("rejects when no definitions are configured", async () => {
    const saved = process.env.FHIR_DEFINITIONS_DIR;
    delete process.env.FHIR_DEFINITIONS_DIR;
    try {
      await expect(validate(BARE_PATIENT)).rejects.toThrow(/no FHIR definitions/);
    } finally {
      if (saved === undefined) delete process.env.FHIR_DEFINITIONS_DIR;
      else process.env.FHIR_DEFINITIONS_DIR = saved;
    }
  });

  it("rejects an unreadable definitions directory", async () => {
    const empty = await mkdtemp(join(tmpdir(), "fhir-defs-"));
    await expect(validate(BARE_PATIENT, empty)).rejects.toThrow(/cannot load definitions/);
  });
});

describe("concurrency", () => {
  it("handles parallel calls", async () => {
    const results = await Promise.all(Array.from({ length: 8 }, () => lint(BARE_PATIENT)));
    expect(results.every((diagnostics) => diagnostics.length === 3)).toBe(true);
  });
});

describe("locations", () => {
  it("populates line, col and offset for located findings", async () => {
    const json = '{\n  "resourceType": "Patient",\n  "bogus": 1\n}';
    const d = (await validate(json)).find((x) => x.path === "Patient.bogus");
    expect(d?.line).toBe(3);
    expect(d?.col).toBeGreaterThan(0);
    expect(d?.offset).toBeGreaterThan(json.indexOf("bogus") - 3);
    expect(d?.offset).toBeLessThan(json.length);
  });

  it("omits location when none is available", async () => {
    const [d] = await validate('{"resourceType": "Nope"}');
    expect(d).toBeDefined();
    expect(d).not.toHaveProperty("line");
    expect(d).not.toHaveProperty("col");
    expect(d).not.toHaveProperty("offset");
    expect(d?.rule).toBeUndefined();
  });
});

describe("definitionsDir", () => {
  const abs = process.env.FHIR_DEFINITIONS_DIR ?? "";
  const json = '{"resourceType": "Patient", "bogus": 1}';

  it("gives the same result for relative and absolute paths", async () => {
    const rel = relative(process.cwd(), abs);
    expect(await validate(json, rel)).toEqual(await validate(json, abs));
    expect(await lint(BARE_PATIENT, undefined, rel)).toEqual(
      await lint(BARE_PATIENT, undefined, abs),
    );
  });

  it("takes precedence over the environment variable", async () => {
    const empty = await mkdtemp(join(tmpdir(), "fhir-defs-"));
    await expect(validate(json, empty)).rejects.toThrow(/cannot load definitions/);
    await expect(validate(json)).resolves.toBeDefined();
  });
});

describe("event loop", () => {
  it("keeps ticking while validating", async () => {
    const big = JSON.stringify({
      resourceType: "Patient",
      name: Array.from({ length: 20000 }, () => ({ family: "X", given: ["a", "b"] })),
    });
    let ticks = 0;
    const timer = setInterval(() => ticks++, 1);
    try {
      await Promise.all(Array.from({ length: 4 }, () => validate(big)));
    } finally {
      clearInterval(timer);
    }
    expect(ticks).toBeGreaterThan(0);
  });
});
