# ADR-002 — The factory loop

**Status:** Accepted
**Date:** 2026-11-27

## Context

`rauth-rs` is built by agents. The author does not write the specs or the code. They approve both.
That only works if the loop makes each approval cheap and makes it mechanically hard for any one
agent's mistake to reach `main`.

Two facts shape the design:

- **CI checks the contract, not whether the contract is right.** A green build proves the code
  matches its tests. It cannot prove the tests encode the RFC. Independence has to come from
  somewhere other than CI.
- **Agents in one model family share blind spots.** Separate contexts catch errors from carried-over
  context, not shared misreadings of an RFC. The only check that nobody in the factory wrote is the
  OpenID conformance suite.

References: Rauthy `AGENTS.md:8` (agents need local human approval and open PRs as drafts) and
`AGENTS.md:521` (final checklist: `cargo fmt`, `cargo clippy --all-targets -- -D warnings`, tests),
`rauthy/justfile:544` (`pre-pr-checks`, one recipe for local and CI), IronAuth
`deploy/conformance/README.md` (fail-closed conformance harness). All verified by reading them.
Paths are relative to `reference/src/`.

## Decisions

### D1 — The spec is an issue, drafted by an agent and approved by the author

Each unit of work is a GitHub issue built from the issue form `.github/ISSUE_TEMPLATE/spec.yml`:

| Field | Content |
|---|---|
| Scope | What the change does, in one paragraph |
| Normative basis | The RFC/OIDC sections it implements, with the MUST/SHOULD lines **quoted** and numbered |
| Interface | The public types and signatures the acceptance tests call |
| Acceptance criteria | One line per check, each traced to a quoted requirement or an ADR decision |
| Out of scope | What a reader might expect that this issue does not do |
| ADR links | Decisions the change depends on or changes |

**Gate 1:** the author adds the `spec-approved` label. No agent starts tests or code on an issue
without it, and CI enforces that: `cargo xtask spec-gate` fails any PR whose issue is not open and
`spec-approved`. Quoting the RFC turns the author's review into comparing text with text instead of
judging prose, and that is the review the rest of the loop depends on.

### D2 — Four agent roles, each with its own context

| Role | Sees | Writes | May not |
|---|---|---|---|
| Spec writer | RFCs, ADRs, references | The issue | Write code or tests |
| Test writer | The approved issue, RFCs | Acceptance tests | See an implementation (there is none yet: it runs first) |
| Implementer | The issue, the acceptance tests, the codebase | Production code, unit tests | Change acceptance tests (D3) |
| Reviewer | The PR diff, the issue, ADRs | Review comments | Edit the branch |

Roles are Claude Code agent definitions in `.claude/agents/` (`spec-writer`, `test-writer`,
`implementer`, `reviewer`), each with only the tools its role needs. Every role starts from a fresh context, never one carried over from another role. The test
writer is separated from the implementer by **ordering**: when the tests are written, there is no
implementation to copy from.

**Gate 2:** the author reviews the code and merges. Agents never merge.

### D3 — Acceptance tests are frozen for the implementer, and must fail first

Acceptance tests live in `crates/<crate>/tests/acceptance/` and land in **their own PR** before the
implementation:

1. **Tests PR** (branch `<N>-acceptance`). Each new test is marked `#[ignore = "acceptance: #N"]`.
   The interface the issue names is added as stubs with `todo!()` bodies, so the tests compile and
   fail. CI requires **every marked test to fail** when run with `--ignored`; a test that passes
   before the implementation exists tests nothing (red before green). A new acceptance test without
   the marker, or one using `#[should_panic]`, which would invert the check, is rejected.
2. **Implementation PR** (branch `<N>-<slug>`). Under `tests/acceptance/`, the only allowed diff is
   removing the issue's own markers. CI rejects any other change, including another issue's marker,
   and fails if a marker for the issue is left behind. With the markers gone, the normal test run
   must pass them.

`cargo xtask acceptance` implements both, comparing `HEAD` with its merge base on `main`.
`xtask/tests/e2e.sh` proves it against deliberate good and bad changes in a scratch repository, and
runs in the gate. Acceptance modules are files (`main.rs`, one file per module), not inline `mod`
blocks, so test paths can be read without a Rust parser.

This is a diff check, not trust in commit trailers. The freeze holds even though every agent
pushes with the same credentials.

### D4 — One merge gate, the same locally and in CI

`just check` runs, in order, and fails closed:

- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets -- -D warnings`, with `clippy::pedantic` on workspace-wide
- `cargo nextest run --workspace`, which includes the storage conformance kit against the `-memory`
  and `-postgres` adapters (ADR-001 D7) once the first storage port exists
- `cargo deny check` (advisories, licences, bans, sources; `deny.toml`)
- the factory checks: the spec gate (D1), the branch name (D5), red before green and the freeze
  (D3), and the end-to-end proof of those checks
- `mkdocs build --strict`, so a doc change with a broken link or orphaned page fails
- `gitleaks` over the full history

CI (`.github/workflows/ci.yml`, job `check`) installs pinned tool versions and runs the same
recipe, so a branch that passes locally passes in CI. `main` is protected: changes arrive only by
PR, `check` is a required status check, history is linear, and force-pushes are refused, for
administrators too. The OpenID conformance
suite joins the gate in D6.

### D5 — Branches name their issue; one worktree per branch

Each issue gets two branches, `<N>-acceptance` for its tests PR and `<N>-<slug>` for its
implementation PR, each checked out in its own git worktree so agents working on different issues
never share a working tree. Work with no acceptance tests (tooling, docs) uses only `<N>-<slug>`.
CI rejects any other branch name. The implementation PR closes the issue (`Closes #N`); the tests PR
refers to it (`Refs #N`).
PRs opened by an agent carry the `agent` label, so the PR list tells factory work from the
author's. Commits carry no AI attribution trailer.

### D6 — The OpenID conformance suite is the external oracle

From ADR-001 phase 3 on, CI runs the OIDF suite against `rauth-allinone`:

- suite images pinned by digest
- a reviewed profile matrix: the basic OP plan gates every merge, the full matrix runs nightly
- a results gate that fails on any non-PASS, with WARNING waivers reviewed and committed
- a runner that exits non-zero if it could not drive a plan

This follows IronAuth `deploy/conformance/`. IronAuth's own README (`README.md:29`) says its runner
is not provisioned yet, so this ADR takes the design, not a proven pipeline. Tracked in #8.

### D7 — Load test against ADR-001 D2

The production phase load-tests the split deployables against D2's estimate (~1,700 refreshes/s).
Either the estimate is confirmed or ADR-001 is amended with the measured figure. Tracked in #9.

## Consequences

- Every feature costs two PRs (tests, then implementation) and two author approvals (spec,
  merge). That is slower than one agent doing everything. It is the price of the author's two
  reviews being the only judgement calls in the loop.
- Agents push as the author, and GitHub does not let a PR's author approve it (*recalled*), so
  "required approvals" cannot be the human gate. The gate is that only the author merges.
- Freezing acceptance tests means a test that turns out to be wrong needs a new tests PR. The
  implementer cannot fix it in place, so a spec mistake surfaces as a blocked PR rather than as a
  quietly edited test.
- Until #8 lands, the loop has no independent oracle. The spec gate carries all of the protocol
  correctness for phases 1 and 2.

## Implementation phases

1. **Loop and gate (#7).** The spec issue form and `spec-approved` label, the four agent
   definitions, the Cargo workspace, `just check`, the CI workflow, the D3 checks, and branch
   protection on `main`. From then on, all work goes through D5.
2. **Conformance (#8).** With ADR-001 phase 3.
3. **Load test (#9).** With ADR-001 phase 7.
