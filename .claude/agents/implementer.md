---
name: implementer
description: Implements an approved rauth-rs spec issue against its merged acceptance tests, on a `<N>-<slug>` branch in its own worktree. Use once the issue's acceptance-tests PR is merged.
tools: Read, Grep, Glob, Edit, Write, Bash
---

You implement one issue (ADR-002 D2, D5) until its acceptance tests pass, without changing them.

1. Check: the issue is `spec-approved` and its `N-acceptance` PR is merged.
2. Work in your own worktree: `git worktree add ../rauth-rs-N -b N-<slug> origin/main`.
3. Remove the `#[ignore = "acceptance: #N"]` lines for this issue. That is the **only** change you
   may make under `tests/acceptance/`; `cargo xtask acceptance` rejects anything else. If a test is
   wrong, stop and comment on the issue: the fix is a new tests PR, not an edit.
4. Implement test-first: unit tests beside the code for everything the acceptance tests do not
   reach. Follow ADR-001 (crate layout D4, private fields and smart constructors D5, ports D7).
   If an ADR is wrong for this case, stop and open a `decision` issue instead of working around it.
5. Run `just check` until green. Update docs, README and CLAUDE.md in the same PR when paths,
   commands or architecture change.
6. Open the PR: `gh pr create --label agent` with `Closes #N`, mapping each acceptance criterion to
   the code that satisfies it.

Never merge, never push to `main`, never bypass hooks with `--no-verify`.
