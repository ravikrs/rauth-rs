//! `cargo xtask`: the factory checks behind `just check` (ADR-002 D3–D5).
//!
//! - `cargo xtask spec-gate [--branch B]`: the branch's issue carries `spec-approved`.
//! - `cargo xtask acceptance [--base REF] [--branch B]`: on `<N>-acceptance`, every new
//!   acceptance test is marked for issue `N` and fails before the implementation exists; on
//!   `<N>-<slug>`, acceptance tests are frozen except for removing issue `N`'s markers.
//!
//! Both check committed state (`HEAD` against the merge base with `--base`), not the working tree.

mod branch;
mod freeze;
mod libtest;
mod scan;

use std::collections::{BTreeMap, HashSet};
use std::env;
use std::path::Path;
use std::process::{Command, ExitCode};

use branch::Branch;
use libtest::Outcome;
use scan::{TestFn, is_marker_line, marker, test_fns};

const USAGE: &str = "usage: cargo xtask <spec-gate|acceptance> [--base REF] [--branch NAME]";
const SPEC_APPROVED: &str = "spec-approved";

fn main() -> ExitCode {
    let v: Vec<u8> = Vec::new();
    if v.len() == 0 {}
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("xtask: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let command = args.next().ok_or(USAGE)?;
    let mut base = "origin/main".to_owned();
    let mut branch = None;
    while let Some(flag) = args.next() {
        let value = args
            .next()
            .ok_or_else(|| format!("{flag} needs a value\n{USAGE}"))?;
        match flag.as_str() {
            "--base" => base = value,
            "--branch" => branch = Some(value),
            _ => return Err(format!("unknown flag {flag}\n{USAGE}")),
        }
    }
    let branch = Branch::parse(&branch.map_or_else(current_branch, Ok)?)?;
    match command.as_str() {
        "spec-gate" => spec_gate(&branch),
        "acceptance" => acceptance(&branch, &base),
        _ => Err(USAGE.to_owned()),
    }
}

/// CI names the PR branch in `GITHUB_HEAD_REF` and a pushed branch in `GITHUB_REF_NAME`.
fn current_branch() -> Result<String, String> {
    for var in ["GITHUB_HEAD_REF", "GITHUB_REF_NAME"] {
        if let Ok(name) = env::var(var)
            && !name.is_empty()
        {
            return Ok(name);
        }
    }
    git(&["rev-parse", "--abbrev-ref", "HEAD"]).map(|s| s.trim().to_owned())
}

fn spec_gate(branch: &Branch) -> Result<(), String> {
    let Some(issue) = branch.issue() else {
        println!("spec-gate: {branch}, nothing to check");
        return Ok(());
    };
    let out = run_cmd(
        "gh",
        &[
            "issue",
            "view",
            &issue.to_string(),
            "--json",
            "state,labels",
            "--jq",
            r#".state + " " + (.labels | map(.name) | join(","))"#,
        ],
    )?;
    let (state, labels) = out.trim().split_once(' ').unwrap_or((out.trim(), ""));
    if state != "OPEN" {
        return Err(format!("spec-gate: issue #{issue} is {state}, not OPEN"));
    }
    if !labels.split(',').any(|l| l == SPEC_APPROVED) {
        return Err(format!(
            "spec-gate: issue #{issue} has no `{SPEC_APPROVED}` label; the author approves the \
             spec before tests or code start (ADR-002 D1)"
        ));
    }
    println!("spec-gate: #{issue} is {SPEC_APPROVED}");
    Ok(())
}

fn acceptance(branch: &Branch, base: &str) -> Result<(), String> {
    let merge_base = git(&["merge-base", base, "HEAD"])?.trim().to_owned();
    let problems = match *branch {
        Branch::Main => {
            println!("acceptance: {branch}, nothing to check");
            return Ok(());
        }
        Branch::Tests { issue } => tests_pr(issue, &merge_base)?,
        Branch::Implementation { issue } => implementation_pr(issue, &merge_base)?,
    };
    if problems.is_empty() {
        println!("acceptance: {branch} passes");
        Ok(())
    } else {
        Err(format!(
            "acceptance: {branch} fails ADR-002 D3:\n  - {}",
            problems.join("\n  - ")
        ))
    }
}

/// An acceptance test located in the tree.
struct Located {
    crate_dir: String,
    file: String,
    module: String,
    test: TestFn,
}

impl Located {
    fn id(&self) -> String {
        if self.module.is_empty() {
            self.test.name.clone()
        } else {
            format!("{}::{}", self.module, self.test.name)
        }
    }
}

fn tests_pr(issue: u32, merge_base: &str) -> Result<Vec<String>, String> {
    let head = acceptance_tests("HEAD")?;
    let old_unmarked: HashSet<(String, String)> = acceptance_tests(merge_base)?
        .into_iter()
        .filter(|t| t.test.pending_issue.is_none())
        .map(|t| (t.file, t.test.name))
        .collect();

    let mut problems = Vec::new();
    let mut pending: BTreeMap<String, Vec<&Located>> = BTreeMap::new();
    for t in &head {
        match t.test.pending_issue {
            None if !old_unmarked.contains(&(t.file.clone(), t.test.name.clone())) => {
                problems.push(format!(
                    "{}: `{}` is a new acceptance test without `{}`",
                    t.file,
                    t.test.name,
                    marker(issue)
                ));
            }
            Some(n) if n == issue => {
                if t.test.should_panic {
                    problems.push(format!(
                        "{}: `{}` uses #[should_panic], which inverts red-before-green",
                        t.file, t.test.name
                    ));
                }
                pending.entry(t.crate_dir.clone()).or_default().push(t);
            }
            _ => {}
        }
    }
    if pending.is_empty() {
        problems.push(format!("no acceptance tests marked `{}`", marker(issue)));
    }
    for (crate_dir, tests) in pending {
        problems.extend(red_before_green(&crate_dir, &tests)?);
    }
    Ok(problems)
}

/// Runs the crate's ignored acceptance tests and requires each of `tests` to fail.
fn red_before_green(crate_dir: &str, tests: &[&Located]) -> Result<Vec<String>, String> {
    let manifest = format!("{crate_dir}/Cargo.toml");
    let output = Command::new("cargo")
        .args(["test", "--manifest-path", &manifest, "--test", "acceptance"])
        .args(["--", "--ignored", "--test-threads", "1"])
        .output()
        .map_err(|e| format!("cannot run cargo: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let outcomes = libtest::outcomes(&stdout);
    if outcomes.is_empty() {
        let tail: Vec<&str> = stderr.lines().rev().take(15).collect();
        let tail: Vec<&str> = tail.into_iter().rev().collect();
        return Ok(vec![format!(
            "{crate_dir}: acceptance tests did not run (they must compile against stubs that \
             fail):\n{}",
            tail.join("\n")
        )]);
    }
    let mut problems = Vec::new();
    for t in tests {
        match outcomes.get(&t.id()) {
            Some(Outcome::Failed) => println!("acceptance: red as required: {}", t.id()),
            Some(Outcome::Passed) => problems.push(format!(
                "{}: `{}` passes before the implementation exists, so it tests nothing",
                t.file,
                t.id()
            )),
            Some(Outcome::Ignored) | None => problems.push(format!(
                "{}: `{}` was not run (inline `mod` blocks are not supported in acceptance files)",
                t.file,
                t.id()
            )),
        }
    }
    Ok(problems)
}

fn implementation_pr(issue: u32, merge_base: &str) -> Result<Vec<String>, String> {
    let diff = git(&[
        "diff",
        "--no-color",
        "--no-renames",
        "--unified=0",
        merge_base,
        "HEAD",
        "--",
        ":(glob)crates/*/tests/acceptance/**",
    ])?;
    let mut problems: Vec<String> = freeze::check(&diff, issue)
        .into_iter()
        .map(|v| format!("{}: {}", v.file, v.detail))
        .collect();
    for file in acceptance_files("HEAD")? {
        let source = git(&["show", &format!("HEAD:{file}")])?;
        for (n, line) in source.lines().enumerate() {
            if is_marker_line(line, issue) {
                problems.push(format!(
                    "{file}:{}: `{}` is still present; the implementation PR removes it",
                    n + 1,
                    marker(issue)
                ));
            }
        }
    }
    Ok(problems)
}

/// Acceptance sources at `rev`: `crates/<crate>/tests/acceptance/**/*.rs`.
fn acceptance_files(rev: &str) -> Result<Vec<String>, String> {
    let listing = git(&["ls-tree", "-r", "--name-only", rev, "--", "crates"])?;
    Ok(listing
        .lines()
        .filter(|p| {
            let parts: Vec<&str> = p.split('/').collect();
            parts.len() >= 5
                && parts[0] == "crates"
                && parts[2] == "tests"
                && parts[3] == "acceptance"
                && Path::new(p).extension().is_some_and(|e| e == "rs")
        })
        .map(str::to_owned)
        .collect())
}

fn acceptance_tests(rev: &str) -> Result<Vec<Located>, String> {
    let mut all = Vec::new();
    for file in acceptance_files(rev)? {
        let source = git(&["show", &format!("{rev}:{file}")])?;
        let parts: Vec<&str> = file.split('/').collect();
        let crate_dir = parts[..2].join("/");
        let module = module_path(&parts[4..]);
        all.extend(test_fns(&source).into_iter().map(|test| Located {
            crate_dir: crate_dir.clone(),
            file: file.clone(),
            module: module.clone(),
            test,
        }));
    }
    Ok(all)
}

/// `main.rs` is the crate root; `a.rs` and `a/mod.rs` are `a`; `a/b.rs` is `a::b`.
fn module_path(rel: &[&str]) -> String {
    let mut segments: Vec<&str> = rel.to_vec();
    if let Some(last) = segments.pop() {
        let stem = last.trim_end_matches(".rs");
        if stem != "main" && stem != "mod" {
            segments.push(stem);
        }
    }
    segments.join("::")
}

fn git(args: &[&str]) -> Result<String, String> {
    run_cmd("git", args)
}

fn run_cmd(program: &str, args: &[&str]) -> Result<String, String> {
    let output = Command::new(program)
        .args(args)
        .output()
        .map_err(|e| format!("cannot run {program}: {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(format!(
            "`{program} {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::module_path;

    #[test]
    fn module_paths_follow_cargo_layout() {
        assert_eq!(module_path(&["main.rs"]), "");
        assert_eq!(module_path(&["clients.rs"]), "clients");
        assert_eq!(module_path(&["clients", "mod.rs"]), "clients");
        assert_eq!(
            module_path(&["clients", "redirect.rs"]),
            "clients::redirect"
        );
    }
}
