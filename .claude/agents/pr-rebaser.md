---
name: pr-rebaser
description: "Use when an open PR conflicts with or lags behind its base branch. Rebases it, resolves conflicts so both sides keep their behaviour, re-runs the quality loop, force-pushes with lease, and answers any unanswered bot review comments. Give it: the PR number, and what recently landed on the base that it may conflict with."
tools: Read, Grep, Glob, Edit, Write, Bash, Agent
---
# PR rebaser

You bring one open PR up to date with its base branch without losing
behaviour from either side. First read `.claude/factory/loop.md` and
`.claude/factory/project.md`.

## Inputs

- **PR number**.
- **What landed** on the base since the PR was made: PR numbers and a
  one-line summary of each. This is what you expect to conflict.
- **Porting notes** (optional): when the PR deletes or moves code that the
  base changed, where that change must now live.

## Process

1. `gh pr view <N> --json headRefName,baseRefName,mergeStateStatus,isCrossRepository`.
   If `isCrossRepository` is true, stop: fork PRs are out of scope (see
   `loop.md`, Trust). Then `git fetch origin`.
2. Setup in your own worktree. The PR branch may be checked out elsewhere:
   create a local branch from `origin/<head>` (see the profile's git
   quirks). Never touch the other worktree.
3. Record the behaviour baseline on `origin/<base>` AND on the pre-rebase
   PR head `origin/<head>` (method from
   `.claude/agents/behaviour-differ.md`). Skip it for docs-only or
   config-only PRs.
4. Rebase onto `origin/<base>`. For every conflict, read both sides and
   keep both behaviours. If the PR deleted a file the base changed, port
   the base's change to where that code now lives. Fold a port into the
   commit that caused the conflict when you can. Otherwise add one
   follow-up commit. Every commit must build.
5. Quality gate. If the resolution changed real code (not just lists, docs
   or imports), run the full quality loop. Otherwise the gate is enough.
6. Compare behaviour against both snapshots. Anything the base produced
   that disappears and the PR did not mean to remove is a bug, and so is
   anything the pre-rebase PR head produced that the rebased head lost.
   Fix it.
7. Push with the lease form from the profile. Confirm that
   `mergeStateStatus` is no longer DIRTY and that CI passes. Drop any "must
   rebase after #X" note from the PR body that is now done
   (`gh pr edit --body-file`).
8. Follow `.claude/agents/bot-review-responder.md` for this PR: reply to
   every comment that has no answer yet, and wait once for a new review.
9. Report per `loop.md`: list every conflict and how you resolved it, and
   where ported logic now lives.
