# fhir-linter

A linter and validator for FHIR R5 resources.

## What it checks

- **Validation:** structure, cardinality, types, terminology, invariants (via FHIRPath).
- **Linting:** style rules you can turn on, off, or re-grade.

Built-in lint rules:

| Rule | Flags |
|------|-------|
| `narrative-absent` | Resource has no narrative text: `text` absent, `text.status` is `empty`, or the div holds only whitespace once tags are stripped and no `<img>` |
| `missing-identifier` | Resource has no identifier (empty arrays count as none); a Bundle only when its type is `document`, `message` or `collection` |
| `missing-meta-profile` | Resource has no `meta.profile` (empty arrays count as none) |

`narrative-absent` is a deliberate superset of the validator's `dom-6`
invariant: a resource without `text` gets both `INVARIANT_dom-6` from
`validate` and `LINT_001` from `lint`, and `LINT_001` also fires on empty
narrative that `dom-6` accepts.

## Install

Each release is published to PyPI (`pip install fhir-linter`) and to
GitHub Packages (`@jonperron/fhir-linter`), and its wheels, sdist and
npm tarball are attached to the GitHub release, for Linux x86_64, macOS
arm64 and Windows x86_64. Installing from GitHub Packages needs
`@jonperron:registry=https://npm.pkg.github.com` in `.npmrc` and a token
with `read:packages`. The `fhir-lint` CLI is not released yet, and
nothing is published to crates.io or npmjs.org. On Linux, the npm
addon needs glibc 2.35 or newer (Ubuntu 22.04, Debian 12, RHEL 10); the
wheels are manylinux2014 (glibc 2.17).

Usage: [Python](bindings/python/README.md) ·
[Node.js](bindings/typescript/README.md).

## Setup

```bash
git clone --recurse-submodules <repo-url>
cd fhir-linter
./scripts/download-definitions.sh   # fetches FHIR R5 definitions, SHA-256 verified
cargo build --workspace
cargo test --workspace
```

## Configure

Create a `fhir-linter.toml`:

```toml
[rules.narrative-absent]
severity = "error"      # fatal | error | warning | information

[rules.missing-identifier]
enabled = false
```

Severities are the FHIR IssueSeverity codes. `hint` is still accepted as an
alias and is reported as `information`.

## Run

```bash
cargo run -p fhir-cli -- [--format text|json|sarif] [--config fhir-linter.toml] \
    [--strict] [--definitions definitions/r5] <FILES|DIRS>...
```

Directories are scanned recursively for `*.json` and `*.xml`. The definitions
directory defaults to `definitions/r5` (or `$FHIR_DEFINITIONS_DIR`).
Exit codes: `0` clean, `1` errors found (warnings too with `--strict`),
`2` tool error (bad arguments, unreadable file, bad config or definitions).

Full reference: [crates/fhir-cli/README.md](crates/fhir-cli/README.md).

## Layout

| Crate | Job |
|-------|-----|
| `fhir-definitions` | Loads R5 StructureDefinitions into a registry |
| `fhir-parser` | JSON and XML to an AST with source spans |
| `fhir-fhirpath` | FHIRPath evaluator |
| `fhir-validator` | Validation checks |
| `fhir-linter` | Configurable lint rules |
| `fhir-cli` | The `fhir-lint` binary |

Specs live in `openspec/specs/`.

## License

See [LICENSE](LICENSE).
