import { defineConfig } from "vitest/config";

export default defineConfig({
  test: { env: { FHIR_DEFINITIONS_DIR: process.env.FHIR_DEFINITIONS_DIR ?? "../../definitions/r5" } },
});
