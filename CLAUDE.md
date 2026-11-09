# FHIR Linter — Claude Guidelines

## Project Overview

Rust workspace implementing a FHIR R5 linter and validator, with Python (PyO3) and
TypeScript (napi-rs, native Node addon) bindings.

## Specifications

Feature specifications live in openspec/specs/. Each subfolder contains a spec.md describing a functional area (e.g., cardinality rules, FHIRPath functions) and a rules.

Always read the relevant spec before implementing or modifying a feature.

## Tech stack

- Rust for core logic and validation rules
- PyO3 for Python bindings
- napi-rs for TypeScript (Node.js) bindings
- clap for CLI argument parsing

### FHIR definitions

- Official artifacts are fetched by `scripts/download-definitions.sh` (SHA-256 verified).
- Never hard-code FHIR R5 resource shapes; always derive from the `Registry`.
- The `Registry` is read-only at runtime; build it once and share via `Arc`.

## Key Files

| File | Purpose |
|------|---------|
| `Cargo.toml` (root) | Workspace definition, shared dependencies |
| `definitions/r5/` | Bundled FHIR R5 JSON artifacts |
| `tests/fixtures/` | Official FHIR test cases (submodule) |
| `scripts/download-definitions.sh` | Fetches & checksums FHIR R5 definitions |
| `fhir-linter.toml` | User-supplied linter rule config (not committed; schema in `crates/fhir-linter/src/config.rs`) |

## Architecture

```
crates/
  fhir-definitions/   # FHIR R5 StructureDefinitions / ValueSets registry
  fhir-parser/        # JSON + XML → typed AST with source spans
  fhir-fhirpath/      # FHIRPath 2.0 evaluator (required for invariants)
  fhir-validator/     # Structural, cardinality, type, terminology, invariant checks
  fhir-linter/        # Configurable lint rules (fhir-linter.toml)
  fhir-cli/           # CLI binary (planned: clap, text / JSON / SARIF output; currently a stub)
bindings/
  python/             # maturin + PyO3
  typescript/         # napi-rs (@napi-rs/cli)
definitions/r5/       # Bundled FHIR R5 JSON artifacts (do not edit manually)
tests/fixtures/       # git submodule → github.com/FHIR/fhir-test-cases
```

## Coding Rules

Coding rules are defined in `.claude/rules/` and loaded automatically by file pattern:

- [Coding style](.claude/rules/coding-style.md): Guidelines for code formatting, organization, and idiomatic patterns.
- [FHIR rules](.claude/rules/fhir.md): The FHIR R5 spec is always right; validator, parser and FHIRPath lessons from the audit rounds.
- [Python coding style](.claude/rules/python-coding-style.md): Conventions for the Python bindings (`bindings/python/`).
- [TypeScript coding style](.claude/rules/typescript-coding-style.md): Conventions for the TypeScript bindings (`bindings/typescript/`).
- [Testing](.claude/rules/testing.md): Best practices for writing tests, TDD workflow, coverage targets, and test types.
- [Commit messages](.claude/rules/commit-messages.md): Conventions for writing clear, consistent commit messages that enhance project history and collaboration.

## Subagents

When completing any task on a Rust file, delegate to subagents for quality assurance:

- **Review Agent** — Use the review agent (`.claude/agents/rust-reviewer.md`) to validate changes against specs and project conventions before finishing
- **Build Agent**: Use the build agent (`.claude/agents/build.md`) to ensure your code compiles and builds correctly.
- **Linting Agent**: Use the linting agent (`.claude/agents/linter.md`) to check for code style, formatting, clippy warnings, and adherence to project conventions.
- **Testing Agent**: Use the testing agent (`.claude/agents/test.md`) to write tests, implement TDD cycles, and verify coverage targets.

Run first the Review Agent to check for correctness, safety, and spec adherence. Then run the others if no issue found.

When completing any task on a Python file, run the Python Reviewer (`.claude/agents/python-reviewer.md`) to check correctness, style, security and conventions before finishing.

When completing any task on a TypeScript file, run the TypeScript Reviewer (`.claude/agents/typescript-reviewer.md`) to check correctness, style, security and conventions before finishing.

When updating a library, use the following subagents:

- **Security Agent**: Use the security agent (`.claude/agents/security.md`) to review code for security vulnerabilities.

When implementing a new feature or modifying an existing one, use the following subagents:

- **OpenSpec Agent** — Use the openspec agent (`.claude/agents/openspec.md`) to update specifications after implementing or modifying features

Specifications must always be up to date at the end of any task that adds, changes, or removes functionality. After completing implementation work, run the OpenSpec subagent to synchronize openspec/specs/ with the current state of the code.

### Factory agents (multi-issue and PR work)

For work spanning several issues or PRs, delegate to these agents instead of
general-purpose ones. They share `.claude/factory/loop.md` (the quality loop)
and read every repo-specific detail from `.claude/factory/project.md`.

| Agent | Use it to |
| --- | --- |
| `issue-driver` | Take one issue from a failing test to a reviewed commit or PR |
| `pr-consolidator` | Merge several branches on the same component into one PR |
| `pr-rebaser` | Rebase a conflicting PR, keeping both sides' behaviour |
| `bot-review-responder` | Verify, fix and reply to every automated review comment |
| `behaviour-differ` | Compare corpus output between two refs to prove what changed |

Group issues that touch the same component into one PR (`pr-consolidator`).
Run each agent with `isolation: "worktree"`, except `behaviour-differ`, which
is read-only.
