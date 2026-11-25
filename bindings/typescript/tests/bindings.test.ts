import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, relative } from "node:path";
import { fileURLToPath } from "node:url";
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

  it.each([
    ["fatal", "fatal"],
    ["hint", "information"],
  ] as const)("outputs config severity %s as %s", async (configured, expected) => {
    const config = `[rules.narrative-absent]\nseverity = "${configured}"`;
    const byRule = new Map((await lint(BARE_PATIENT, config)).map((d) => [d.rule, d]));
    expect(byRule.get("narrative-absent")?.severity).toBe(expected);
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

describe("extraDefinitions", () => {
  const definitionsDir = process.env.FHIR_DEFINITIONS_DIR;
  const extraDir = fileURLToPath(new URL("./fixtures/extra", import.meta.url));
  const profileFile = join(extraDir, "patient-with-birthdate.json");
  const profiled = JSON.stringify({
    resourceType: "Patient",
    meta: { profile: ["http://example.org/fhir/StructureDefinition/patient-with-birthdate"] },
  });
  const findings = (diagnostics: readonly { code: string; path: string }[]): string[] =>
    diagnostics.map((d) => `${d.code} ${d.path}`);

  it.each([
    ["file", profileFile],
    ["directory", extraDir],
  ])("enforces a supplied profile given as a %s", async (_kind, extra) => {
    const without = findings(await validate(profiled, definitionsDir));
    expect(without).toContain("PROFILE_001 Patient.meta.profile[0]");
    expect(without).not.toContain("CARDINALITY_001 Patient.birthDate");

    const found = findings(await validate(profiled, definitionsDir, [extra]));
    expect(found).toContain("CARDINALITY_001 Patient.birthDate");
    expect(found).not.toContain("PROFILE_001 Patient.meta.profile[0]");
  });

  it("changes nothing when empty", async () => {
    expect(await validate(profiled, definitionsDir, [])).toEqual(
      await validate(profiled, definitionsDir),
    );
    expect(await lint(profiled, undefined, definitionsDir, [])).toEqual(
      await lint(profiled, undefined, definitionsDir),
    );
  });

  it.each([
    ["validate", (extra: string[]) => validate(BARE_PATIENT, definitionsDir, extra)],
    ["lint", (extra: string[]) => lint(BARE_PATIENT, undefined, definitionsDir, extra)],
  ] as const)("%s names the file, not its content, on a load error", async (_name, call) => {
    const dir = await mkdtemp(join(tmpdir(), "fhir-extra-"));
    const broken = join(dir, "broken.json");
    await writeFile(broken, '{"resourceType": "StructureDefinition", SECRET-CONTENT');
    const error = await call([broken]).then(
      () => undefined,
      (e: unknown) => e,
    );
    expect(error).toBeInstanceOf(Error);
    const message = error instanceof Error ? error.message : "";
    expect(message).toMatch(/^cannot load extra definitions: /);
    expect(message).toContain(broken);
    expect(message).not.toContain("SECRET");
  });

  it("names a missing path", async () => {
    const missing = join(tmpdir(), "fhir-extra-does-not-exist.json");
    await expect(validate(BARE_PATIENT, definitionsDir, [missing])).rejects.toThrow(
      `cannot load extra definitions: `,
    );
    await expect(validate(BARE_PATIENT, definitionsDir, [missing])).rejects.toThrow(missing);
  });
});
