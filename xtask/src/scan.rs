//! Finds test functions in acceptance-test sources, with the attributes the gate cares about.
//!
//! A line scanner, not a Rust parser. It understands what acceptance tests are allowed to look
//! like: attributes on their own lines (possibly spanning several), then the `fn`. Inline
//! `mod name { … }` blocks are not tracked; acceptance modules are files.

/// The marker a pending acceptance test carries until its implementation lands (ADR-002 D3).
pub fn marker(issue: u32) -> String {
    format!("#[ignore = \"acceptance: #{issue}\"]")
}

/// A test function found in a source file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TestFn {
    /// The function name.
    pub name: String,
    /// The issue named by an `acceptance: #N` ignore reason, if any.
    pub pending_issue: Option<u32>,
    /// Whether the test carries `#[should_panic]`, which inverts pass and fail.
    pub should_panic: bool,
}

/// Returns every test function in `source`.
pub fn test_fns(source: &str) -> Vec<TestFn> {
    let mut found = Vec::new();
    let mut attrs: Vec<String> = Vec::new();
    let mut partial: Option<String> = None;

    for line in source.lines() {
        let trimmed = line.trim();
        if let Some(mut text) = partial.take() {
            text.push(' ');
            text.push_str(trimmed);
            if brackets_balanced(&text) {
                attrs.push(text);
            } else {
                partial = Some(text);
            }
            continue;
        }
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        if trimmed.starts_with("#[") {
            if brackets_balanced(trimmed) {
                attrs.push(trimmed.to_owned());
            } else {
                partial = Some(trimmed.to_owned());
            }
            continue;
        }
        if let Some(name) = fn_name(trimmed)
            && attrs.iter().any(|a| is_test_attr(a))
        {
            found.push(TestFn {
                name,
                pending_issue: attrs.iter().find_map(|a| pending_issue(a)),
                should_panic: attrs.iter().any(|a| attr_path(a) == "should_panic"),
            });
        }
        attrs.clear();
    }
    found
}

/// Whether `line`, trimmed, is exactly the marker for `issue`.
pub fn is_marker_line(line: &str, issue: u32) -> bool {
    line.trim() == marker(issue)
}

fn brackets_balanced(text: &str) -> bool {
    let open = text.matches('[').count();
    let close = text.matches(']').count();
    open <= close
}

/// The attribute's path: `tokio::test` for `#[tokio::test(flavor = "multi_thread")]`.
fn attr_path(attr: &str) -> &str {
    let inner = attr.trim_start_matches("#[").trim_end_matches(']');
    let end = inner.find(['(', '=', ' ']).unwrap_or(inner.len());
    &inner[..end]
}

fn is_test_attr(attr: &str) -> bool {
    let path = attr_path(attr);
    path == "test" || path.ends_with("::test")
}

fn pending_issue(attr: &str) -> Option<u32> {
    if attr_path(attr) != "ignore" {
        return None;
    }
    let reason = attr.split_once('=')?.1;
    let reason = reason.trim().trim_end_matches(']').trim().trim_matches('"');
    reason.strip_prefix("acceptance: #")?.parse().ok()
}

fn fn_name(line: &str) -> Option<String> {
    let at = if line.starts_with("fn ") {
        0
    } else {
        line.find(" fn ")? + 1
    };
    let rest = &line[at + 3..];
    let end = rest.find(|c: char| !(c.is_alphanumeric() || c == '_'))?;
    (end > 0).then(|| rest[..end].to_owned())
}

#[cfg(test)]
mod tests {
    use super::{TestFn, is_marker_line, marker, test_fns};

    #[test]
    fn finds_tests_and_their_markers() {
        let src = r#"
use something;

#[test]
#[ignore = "acceptance: #12"]
fn rejects_unknown_redirect_uri() {}

#[tokio::test(flavor = "multi_thread")]
async fn issues_token() {}

#[test]
#[should_panic(expected = "boom")]
#[ignore = "acceptance: #3"]
pub fn panics() {}

fn helper() {}

#[test]
#[ignore = "slow"]
fn slow() {}
"#;
        assert_eq!(
            test_fns(src),
            vec![
                TestFn {
                    name: "rejects_unknown_redirect_uri".into(),
                    pending_issue: Some(12),
                    should_panic: false
                },
                TestFn {
                    name: "issues_token".into(),
                    pending_issue: None,
                    should_panic: false
                },
                TestFn {
                    name: "panics".into(),
                    pending_issue: Some(3),
                    should_panic: true
                },
                TestFn {
                    name: "slow".into(),
                    pending_issue: None,
                    should_panic: false
                },
            ]
        );
    }

    #[test]
    fn handles_attributes_split_across_lines() {
        let src = "#[test]\n#[ignore =\n    \"acceptance: #9\"]\nfn split() {}\n";
        assert_eq!(test_fns(src)[0].pending_issue, Some(9));
    }

    #[test]
    fn attributes_do_not_leak_past_other_items() {
        let src = "#[test]\nconst X: u8 = 1;\nfn not_a_test() {}\n";
        assert_eq!(test_fns(src), []);
    }

    #[test]
    fn marker_lines_match_exactly() {
        assert_eq!(marker(4), "#[ignore = \"acceptance: #4\"]");
        assert!(is_marker_line("    #[ignore = \"acceptance: #4\"]  ", 4));
        assert!(!is_marker_line("#[ignore = \"acceptance: #44\"]", 4));
        assert!(!is_marker_line(
            "#[ignore = \"acceptance: #4\"] fn x() {}",
            4
        ));
    }
}
