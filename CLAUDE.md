# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

`rauth-rs` is an OAuth 2.0 / OpenID Connect provider in Rust, and the vehicle for learning how to
run a software factory: agents write the specs, acceptance tests and code; the author approves each
spec and merges each PR; the OpenID conformance suite is the objective finish line. The loop is
`docs/adr-002-factory-loop.md`.

## Status

Cargo workspace with the factory tooling (`xtask/`); no product crates yet. **Read
`docs/adr-001-architecture.md` first** (contexts, deployables, scaling model, crate layout), then
`docs/adr-002-factory-loop.md` (how work moves).

Layout: `crates/` (product crates, `rauth-*`), `xtask/` (factory checks), `docs/` (ADRs, MkDocs,
deployed to GitHub Pages), `reference/` (survey and clone script), `.claude/agents/` (the four
roles), `.github/` (CI, docs deploy, spec issue form, PR template).

## Commands

Rust comes from `rust-toolchain.toml`; tools are `just`, `cargo-nextest`, `cargo-deny`, `gitleaks`
and `mkdocs-material` (`requirements-docs.txt`).

- `just check`: the whole merge gate, exactly as CI runs it. Run before every push.
- `just fmt` / `just clippy` / `just test` / `just deny` / `just docs` / `just secrets`: one gate step.
- `cargo nextest run -p <crate>`: one crate's tests.
- `just factory`: spec gate and acceptance checks for the current branch against `origin/main`
  (override with `BASE=<ref>`); they read committed state, so commit first.
- `just test-factory`: the end-to-end proof that the acceptance checks accept and reject correctly.

## How work moves (ADR-002)

`main` only changes through PRs that pass `check`. Per issue: `spec-writer` drafts it, the author
labels it `spec-approved`, `test-writer` opens `<N>-acceptance` with failing tests marked
`#[ignore = "acceptance: #N"]`, `implementer` opens `<N>-<slug>` in its own worktree and may only
remove those markers under `tests/acceptance/`, `reviewer` comments, the author merges. Agent PRs
carry the `agent` label. Agents never merge or push to `main`.

## Work tracking

GitHub Issues on `ravikrs/rauth-rs` are the backlog; the **rauth-rs** GitHub Project is the board.
`backlog.md` is only a pointer. At session start, `gh issue list -R ravikrs/rauth-rs` replaces
reading `backlog.md`. Reference the issue in the commit that resolves it (`Closes #N`), and
record `decision` issues in the ADR before closing them. `gh` authenticates with `GH_TOKEN` from
`.claude/settings.local.json` (a classic PAT with `repo`, `project` and `read:org`; fine-grained
tokens cannot reach user-owned Projects).

## Provenance

Justify each design decision from the RFCs and open-source references, with cited paths — a
reference is what makes a decision arguable. Never commit real hostnames, client identifiers or
credentials.

## References

Findings and file anchors live in `reference/README.md`. `reference/clone-refs.sh` shallow-clones
the references into gitignored `reference/src/`; cite paths relative to it.

- **Rauthy** (`sebadob/rauthy`, Apache-2.0) — protocol coverage (token exchange, DPoP,
  back-channel logout, dynamic client registration) and its `AGENTS.md` as a model for running
  agents on a repo. Not a model for domain modelling or storage: it has no storage port.
- **Kanidm — do not read or cite.** Its `AGENTS.md` asks that AI tools not use the codebase as a
  reference. Honour that opt-out, and check every new reference for one before reading it.
- **Hydra, Kratos, Dex, Zitadel** (Go) — authoritative for protocol and domain design; cloned
  by the same script.

## Conventions

- Repo is `rauth-rs`; workspace crates are `rauth-*`, laid out per ADR-001 D4
  (`rauth-<ctx>-domain`, `-postgres`, `-memory`, `-http`, `-api`; binaries under `bins/`).
- Every claim about a reference cites a file path, and says whether it was verified, inferred or
  recalled.
- PRs opened by an agent carry the `agent` label, so factory work can be told from the author's.
