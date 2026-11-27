#!/usr/bin/env bash
# End-to-end check of `cargo xtask acceptance` (ADR-002 D3) against deliberate good and bad
# changes, in a scratch repository with a demo crate. Run by `just test-factory`.
set -uo pipefail
SRC=$(cd "$(dirname "$0")/../.." && pwd)
XT=${XTASK_BIN:-$SRC/target/debug/xtask}
W=$(mktemp -d)/repo; mkdir -p "$W" && cd "$W"
cp "$SRC/rust-toolchain.toml" "$W/"
printf '[workspace]\nresolver = "3"\nmembers = ["crates/*"]\n' > "$W/Cargo.toml"; mkdir -p crates/demo/src crates/demo/tests/acceptance
cat > crates/demo/Cargo.toml <<'T'
[package]
name = "demo"
version = "0.0.0"
edition = "2024"
publish = false
T
echo 'pub fn add(a: u8, b: u8) -> u8 { let _ = (a, b); todo!() }' > crates/demo/src/lib.rs
printf "/target/\n" > .gitignore
git init -q -b main && git add -A && git -c user.name=e2e -c user.email=e2e@example.invalid -c commit.gpgsign=false commit -qm base
pass=0; fail=0
expect() { # expect <ok|reject> <label> <branch>
  out=$("$XT" acceptance --base main --branch "$3" 2>&1); code=$?
  if { [ "$1" = ok ] && [ $code -eq 0 ]; } || { [ "$1" = reject ] && [ $code -ne 0 ]; }; then
    echo "PASS  $2"; pass=$((pass+1))
  else
    echo "FAIL  $2 (exit $code)"; echo "$out" | sed 's/^/      /'; fail=$((fail+1))
  fi
  [ "$1" = reject ] && echo "$out" | grep -E '^\s+- ' | head -2 | sed 's/^/      /'
}
G="git -c user.name=e2e -c user.email=e2e@example.invalid -c commit.gpgsign=false"
commit() { git add -A && $G commit -qm "$1"; }

# Tests PR, good: marked test against a todo!() stub fails.
git switch -qc 5-acceptance
cat > crates/demo/tests/acceptance/main.rs <<'T'
#[test]
#[ignore = "acceptance: #5"]
fn adds() { assert_eq!(demo::add(2, 2), 4); }
T
commit tests; expect ok "tests PR: marked test fails against stub" 5-acceptance

# Tests PR, vacuous: a marked test that passes without an implementation.
git switch -qc 5-acceptance-vacuous-tmp main; git checkout -q 5-acceptance -- crates
cat >> crates/demo/tests/acceptance/main.rs <<'T'
#[test]
#[ignore = "acceptance: #5"]
fn vacuous() { assert!(true); }
T
commit vacuous; expect reject "tests PR: vacuous test that passes" 5-acceptance

# Tests PR, unmarked: new acceptance test without the marker.
git switch -qc unmarked-tmp 5-acceptance
printf '#[test]\nfn unmarked() { assert_eq!(demo::add(1, 1), 2); }\n' >> crates/demo/tests/acceptance/main.rs
commit unmarked; expect reject "tests PR: new test without marker" 5-acceptance

# Tests PR, no tests at all for the issue.
git switch -qc empty-tmp main; expect reject "tests PR: no marked tests" 5-acceptance

# Merge the good tests PR into main.
git switch -q main && git merge -q --ff-only 5-acceptance

# Implementation PR, good: implement, remove own marker.
git switch -qc 5-add
echo 'pub fn add(a: u8, b: u8) -> u8 { a + b }' > crates/demo/src/lib.rs
perl -ni -e 'print unless /acceptance: #5/' crates/demo/tests/acceptance/main.rs
commit impl; expect ok "impl PR: implements and removes own marker" 5-add
(cd "$W" && cargo test -q --manifest-path crates/demo/Cargo.toml --test acceptance 2>&1 | grep -q 'test result: ok. 1 passed') && echo "PASS  impl PR: acceptance test now passes" || { echo "FAIL  acceptance test does not pass after impl"; fail=$((fail+1)); }

# Implementation PR, edited assertion.
git switch -qc 5-edit main
echo 'pub fn add(a: u8, b: u8) -> u8 { a + b + 1 }' > crates/demo/src/lib.rs
perl -pi -e 's/acceptance: #5//; s/, 4\)/, 5)/' crates/demo/tests/acceptance/main.rs
perl -ni -e 'print unless /^#\[ignore = ""\]$/' crates/demo/tests/acceptance/main.rs
commit edit; expect reject "impl PR: edits an acceptance assertion" 5-edit

# Implementation PR, marker left in place.
git switch -qc 5-left main
echo 'pub fn add(a: u8, b: u8) -> u8 { a + b }' > crates/demo/src/lib.rs
commit left; expect reject "impl PR: leaves its marker" 5-left

# Implementation PR for another issue removes #5's marker.
git switch -qc 6-other main
perl -ni -e 'print unless /acceptance: #5/' crates/demo/tests/acceptance/main.rs
commit other; expect reject "impl PR: removes another issue's marker" 6-other

echo; echo "e2e: $pass passed, $fail failed"; rm -rf "$(dirname "$W")"; [ $fail -eq 0 ]
