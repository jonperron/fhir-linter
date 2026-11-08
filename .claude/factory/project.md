# Factory project profile — fhir-linter

The factory agents (`issue-driver`, `pr-consolidator`, `pr-rebaser`,
`bot-review-responder`, `behaviour-differ`) are repo-agnostic. Everything
specific to this repository lives here. To reuse the agents elsewhere, copy
`.claude/agents/{issue-driver,pr-consolidator,pr-rebaser,bot-review-responder,behaviour-differ}.md`
and write a new copy of this file with the same headings.

## Repository

- GitHub: `jonperron/fihr-linter`
- Base branch: `main` (never commit on it, never merge into it locally)
- Conventions: `CLAUDE.md`, `.claude/rules/*.md`
- Commit messages: `.claude/rules/commit-messages.md`
- Commit trailer: the attribution lines given by the harness for this
  session (for example `Co-Authored-By: ...`). PR body footer: the same
  source.

## Git quirks

- Git binary: `/usr/bin/git`. The `rtk` hook rewrites `git` and the
  worktree guard rejects the rewritten form. Every `git` command written
  in an agent file or in `loop.md` means `/usr/bin/git` here, including
  the setup commands below.
- PR source: same-repository branches only (see `loop.md`, trust).
- A branch checked out in another worktree cannot be checked out again.
  To update a PR whose branch lives elsewhere, work on a new local branch
  from `origin/<branch>` and push with
  `--force-with-lease=<branch>:origin/<branch> origin <local>:<branch>`.

## Worktree setup

Run in every fresh worktree, before building:

```bash
for f in "$PRIMARY"/definitions/r5/*.json "$PRIMARY"/definitions/r5/*.zip \
         "$PRIMARY"/definitions/r5/*.sentinel; do
  ln -sf "$f" definitions/r5/
done
git submodule update --init tests/fixtures   # /usr/bin/git, see above
```

`$PRIMARY` is the primary checkout: the first path of `git worktree list`.
The definition files are untracked: never commit them.

## Before every commit

- `/usr/bin/git checkout Cargo.lock` if the only change is a version bump
  produced by the build (the lockfile on `main` can lag behind a release).
  Commit `Cargo.lock` only when the task changes dependencies.
- `git status` must show no symlinked definitions or fixture files staged.

## Quality gate

All must pass:

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

When bindings are touched, also:

- Python: `cd bindings/python && tox -e py312`
- TypeScript: `cd bindings/typescript && npm run build:debug && npx vitest run`

Say so in the report if a toolchain is missing; do not claim it passed.

## Reviewer routing

| Changed files | Reviewer subagent |
| --- | --- |
| `*.rs` | `rust-reviewer` |
| `bindings/python/**` | `python-reviewer` |
| `bindings/typescript/**` | `typescript-reviewer` |
| parsers, input handling, dependencies | `security` |

Severity scale used by the loop: CRITICAL and MAJOR block; MINOR and NIT do
not.

## Spec sync

Behaviour or API changes must update `openspec/specs/`: run the `openspec`
subagent. Known unrelated failures to leave alone: `fhir-linter`,
`fhir-python-bindings`, `fhir-xml-parser`.

## Automated PR reviewer

- Bot login: `sourcery-ai[bot]` (appears as `sourcery-ai` in some APIs)
- It reviews every new PR within a few minutes; it rarely re-reviews a
  force-push.
- Review comments: `gh api --paginate repos/jonperron/fihr-linter/pulls/<N>/comments`
  (and `.../reviews`). Always pass `--paginate`; the default page holds 30.
- Reply: `gh api repos/jonperron/fihr-linter/pulls/<N>/comments/<id>/replies -f body=...`

## Behaviour baseline

- Corpus: every `*.json` under `tests/fixtures/`
  (one copy, shared by every ref; for example the primary checkout's
  `tests/fixtures/`)
- Output to compare: the validator's diagnostics as `severity`, `code`,
  `path` and `message`, one line each, per input file, in emission order.
  The set key is `code` + `path`.
- Harness: a throwaway binary in the scratchpad with a path dependency on
  the checkout under test. It loads the real `Registry` from
  `definitions/r5/` and calls `fhir_validator::validate`. Build it once
  per ref. Never commit it.
- Secondary metric: the manifest pass rate printed by
  `cargo test -p fhir-validator --test manifest_tests -- --nocapture`
