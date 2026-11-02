import { mkdtemp } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
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

  it("rejects invalid TOML", async () => {
    await expect(lint(BARE_PATIENT, "[rules")).rejects.toThrow(/invalid lint config/);
  });
});

describe("errors", () => {
  it.each([validate, lint])("rejects malformed JSON", async (fn) => {
    await expect(fn("{not json")).rejects.toThrow(/parse error/);
  });

  it("rejects when no definitions are configured", async () => {
    const saved = process.env.FHIR_DEFINITIONS_DIR;
    delete process.env.FHIR_DEFINITIONS_DIR;
    try {
      await expect(validate(BARE_PATIENT)).rejects.toThrow(/no FHIR definitions/);
    } finally {
      process.env.FHIR_DEFINITIONS_DIR = saved;
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
