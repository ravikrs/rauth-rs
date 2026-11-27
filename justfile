# The merge gate (ADR-002 D4). CI runs `just check`; run it before pushing.
# Factory checks compare HEAD with BASE (default origin/main) and check committed state.

base := env("BASE", "origin/main")

# The whole gate, in order; stops at the first failure.
check: fmt clippy test deny factory docs secrets

fmt:
    cargo fmt --all --check

clippy:
    cargo clippy --workspace --all-targets --locked -- -D warnings

test:
    cargo nextest run --workspace --locked

deny:
    cargo deny --locked check

# Spec approved, acceptance tests red before green and frozen after (ADR-002 D1, D3).
factory: test-factory
    cargo xtask spec-gate
    cargo xtask acceptance --base {{base}}

# The factory checks against deliberate good and bad changes in a scratch repository.
test-factory:
    cargo build --quiet --locked --package xtask
    ./xtask/tests/e2e.sh

docs:
    mkdocs build --strict --site-dir target/site

secrets:
    gitleaks git --redact --exit-code 1 .
