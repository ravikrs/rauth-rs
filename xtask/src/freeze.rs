//! The acceptance freeze (ADR-002 D3): in an implementation PR, the only change allowed under
//! `tests/acceptance/` is removing the issue's own `#[ignore = "acceptance: #N"]` lines.

use crate::scan::is_marker_line;

/// One change the freeze forbids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub file: String,
    pub detail: String,
}

/// Checks a `git diff --unified=0 --no-renames` of the acceptance paths against the freeze.
pub fn check(diff: &str, issue: u32) -> Vec<Violation> {
    let mut violations = Vec::new();
    let mut file = String::new();

    for line in diff.lines() {
        if let Some(rest) = line.strip_prefix("diff --git ") {
            rest.rsplit(" b/")
                .next()
                .unwrap_or(rest)
                .clone_into(&mut file);
            continue;
        }
        let mut flag = |detail: String| {
            violations.push(Violation {
                file: file.clone(),
                detail,
            });
        };
        if line.starts_with("new file mode") {
            flag("acceptance file added".into());
        } else if line.starts_with("deleted file mode") {
            flag("acceptance file deleted".into());
        } else if line.starts_with("old mode") {
            flag("file mode changed".into());
        } else if line.starts_with("+++ ") || line.starts_with("--- ") || line.starts_with("@@") {
        } else if let Some(added) = line.strip_prefix('+') {
            flag(format!("line added: {}", added.trim()));
        } else if let Some(removed) = line.strip_prefix('-')
            && !is_marker_line(removed, issue)
        {
            flag(format!("line removed: {}", removed.trim()));
        }
    }
    violations
}

#[cfg(test)]
mod tests {
    use super::check;

    const HEADER: &str = "diff --git a/crates/c/tests/acceptance/main.rs b/crates/c/tests/acceptance/main.rs\n\
                          index 1111111..2222222 100644\n\
                          --- a/crates/c/tests/acceptance/main.rs\n\
                          +++ b/crates/c/tests/acceptance/main.rs\n";

    #[test]
    fn allows_removing_own_markers() {
        let diff = format!("{HEADER}@@ -4 +3,0 @@\n-#[ignore = \"acceptance: #12\"]\n");
        assert_eq!(check(&diff, 12), []);
    }

    #[test]
    fn rejects_removing_another_issues_marker() {
        let diff = format!("{HEADER}@@ -4 +3,0 @@\n-#[ignore = \"acceptance: #13\"]\n");
        assert_eq!(check(&diff, 12).len(), 1);
    }

    #[test]
    fn rejects_edited_assertions() {
        let diff = format!(
            "{HEADER}@@ -9 +9 @@\n-    assert_eq!(code, 400);\n+    assert_eq!(code, 200);\n"
        );
        let v = check(&diff, 12);
        assert_eq!(v.len(), 2);
        assert!(
            v.iter()
                .all(|v| v.file == "crates/c/tests/acceptance/main.rs")
        );
    }

    #[test]
    fn rejects_added_and_deleted_files() {
        let diff = "diff --git a/crates/c/tests/acceptance/new.rs b/crates/c/tests/acceptance/new.rs\n\
                    new file mode 100644\n\
                    --- /dev/null\n\
                    +++ b/crates/c/tests/acceptance/new.rs\n\
                    @@ -0,0 +1 @@\n\
                    +fn x() {}\n\
                    diff --git a/crates/c/tests/acceptance/old.rs b/crates/c/tests/acceptance/old.rs\n\
                    deleted file mode 100644\n";
        let details: Vec<_> = check(diff, 1).into_iter().map(|v| v.detail).collect();
        assert_eq!(
            details,
            [
                "acceptance file added",
                "line added: fn x() {}",
                "acceptance file deleted"
            ]
        );
    }

    #[test]
    fn empty_diff_passes() {
        assert_eq!(check("", 1), []);
    }
}
