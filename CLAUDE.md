# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

`rauth-rs` is an OAuth 2.0 / OpenID Connect provider in Rust, and the vehicle for learning how to
run a software factory: agents write the specs, acceptance tests and code; the author approves each
spec and merges each PR; the OpenID conformance suite is the objective finish line. The loop is
`docs/adr-002-factory-loop.md`.

## Status

Pre-code: no Cargo workspace, no build or test commands yet. **Read
`docs/adr-001-architecture.md` first** — contexts, deployables, scaling model, crate layout. When the
workspace lands, add its build/lint/test commands here. Work follows the ADR-002 loop: four agent
roles, acceptance tests frozen for the implementer, one branch and worktree per issue once branch
protection is on.

Layout: `docs/` (ADRs, served by `mkdocs.yml`, deployed to GitHub Pages), `reference/` (survey and
clone script), `.github/workflows/` (`docs` builds and deploys the site, `gitleaks` scans history).

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
