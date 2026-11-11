# @jonperron/fhir-linter

Node.js bindings for the FHIR R5 linter and validator: a native addon with
TypeScript types. Requires Node.js 22+. ES module only.

## Install

The package is published to GitHub Packages, not npmjs.org. Point the scope
at it in `.npmrc` and use a token with `read:packages`:

```ini
@jonperron:registry=https://npm.pkg.github.com
//npm.pkg.github.com/:_authToken=${GITHUB_TOKEN}
```

```bash
npm install @jonperron/fhir-linter
```

Each GitHub release also attaches the tarball
(`npm install ./jonperron-fhir-linter-<version>.tgz`). Prebuilt binaries cover
Linux x86_64 (glibc 2.35+: Ubuntu 22.04, Debian 12, RHEL 10), macOS arm64
and Windows x86_64.

## Quick start

```ts
import { lint, validate } from "@jonperron/fhir-linter";

const json = JSON.stringify({ resourceType: "Patient", id: "p1" });

for (const d of [...(await validate(json)), ...(await lint(json))]) {
  console.log(d.severity, d.code, d.path, d.message);
}
```

Both functions take the resource as a JSON **string**, not an object. XML
input is not supported by the bindings.

## FHIR definitions

The addon does not bundle the FHIR R5 definitions (about 60 MB). Download
them once with `scripts/download-definitions.sh` from the
[repository](https://github.com/jonperron/fihr-linter), then either pass
`definitionsDir` to each call or set `FHIR_DEFINITIONS_DIR`. Without them,
calls reject.

The first call for a directory loads the definitions, which takes a few
seconds; later calls in the same process reuse them. The work runs on a
worker thread, so it does not block the event loop.

`definitionsDir` is read from disk: only pass paths you trust.

## API

### `validate(json: string, definitionsDir?: string): Promise<Diagnostic[]>`

Checks structure, cardinality, types, terminology and invariants (FHIRPath)
against the R5 definitions.

### `lint(json: string, configToml?: string, definitionsDir?: string): Promise<Diagnostic[]>`

Runs the lint rules. `configToml` is the **content** of a `fhir-linter.toml`
file, not a path:

```ts
import { readFile } from "node:fs/promises";

const config = await readFile("fhir-linter.toml", "utf8");
const diagnostics = await lint(json, config);
```

```toml
[rules.narrative-absent]
severity = "error"      # fatal | error | warning | information (hint = information)

[rules.missing-identifier]
enabled = false
```

To pass `definitionsDir` without a config, use `lint(json, undefined, dir)`.

Built-in rules, all `warning` by default:

| Rule | Code | Flags |
|------|------|-------|
| `narrative-absent` | `LINT_001` | No narrative text |
| `missing-identifier` | `LINT_002` | No identifier |
| `missing-meta-profile` | `LINT_003` | No `meta.profile` |

### `Diagnostic`

| Field | Type | Meaning |
|-------|------|---------|
| `severity` | `string` | `fatal`, `error`, `warning` or `information` |
| `code` | `string` | Stable code, e.g. `STRUCTURE_000`, `INVARIANT_dom-6`, `LINT_001` |
| `message` | `string` | Human-readable description |
| `path` | `string` | Element path in the resource |
| `line` | `number \| undefined` | 1-indexed line |
| `col` | `number \| undefined` | 1-indexed column, in UTF-8 bytes |
| `offset` | `number \| undefined` | 0-indexed offset, in UTF-8 bytes |
| `rule` | `string \| undefined` | Lint rule id; absent for validator findings |

## Locations

`col` and `offset` count UTF-8 bytes, not UTF-16 string units: after
non-ASCII text, `json.slice(d.offset)` lands too far. Index the bytes
instead: `Buffer.from(json).subarray(d.offset)`. `line`, `col` and `offset`
are all absent without a location.

## Errors

The returned promise rejects with an `Error` when:

- the JSON cannot be parsed;
- the lint config is invalid (`invalid lint config: ...`) or names an
  unknown rule;
- no definitions directory is given (`no FHIR definitions: ...`);
- the definitions directory cannot be loaded (`cannot load definitions: ...`).
