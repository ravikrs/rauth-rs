//! Branch naming (ADR-002 D5): every change is tied to an issue by its branch name.

use std::fmt;

/// What a branch is for, derived from its name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Branch {
    /// `main`: post-merge runs, where the per-PR checks do not apply.
    Main,
    /// `<N>-acceptance`: the tests PR for issue `N`.
    Tests { issue: u32 },
    /// `<N>-<slug>`: the implementation PR for issue `N`.
    Implementation { issue: u32 },
}

impl Branch {
    /// Classifies a branch name.
    ///
    /// # Errors
    ///
    /// Returns an error naming the expected forms when `name` is neither `main` nor
    /// `<N>-<slug>`.
    pub fn parse(name: &str) -> Result<Self, String> {
        if name == "main" {
            return Ok(Self::Main);
        }
        let invalid = || {
            format!(
                "branch `{name}` does not name an issue: use `<N>-acceptance` for a tests PR or \
                 `<N>-<slug>` for an implementation PR (ADR-002 D5)"
            )
        };
        let (number, slug) = name.split_once('-').ok_or_else(invalid)?;
        if number.is_empty() || !number.bytes().all(|b| b.is_ascii_digit()) || slug.is_empty() {
            return Err(invalid());
        }
        let issue = number.parse().map_err(|_| invalid())?;
        if issue == 0 {
            return Err(invalid());
        }
        Ok(if slug == "acceptance" {
            Self::Tests { issue }
        } else {
            Self::Implementation { issue }
        })
    }

    /// The issue the branch belongs to, if any.
    pub fn issue(&self) -> Option<u32> {
        match self {
            Self::Main => None,
            Self::Tests { issue } | Self::Implementation { issue } => Some(*issue),
        }
    }
}

impl fmt::Display for Branch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Main => f.write_str("main"),
            Self::Tests { issue } => write!(f, "tests PR for #{issue}"),
            Self::Implementation { issue } => write!(f, "implementation PR for #{issue}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Branch;

    #[test]
    fn classifies_valid_names() {
        assert_eq!(Branch::parse("main"), Ok(Branch::Main));
        assert_eq!(
            Branch::parse("12-acceptance"),
            Ok(Branch::Tests { issue: 12 })
        );
        assert_eq!(
            Branch::parse("12-client-aggregate"),
            Ok(Branch::Implementation { issue: 12 })
        );
        assert_eq!(
            Branch::parse("7-acceptance-freeze"),
            Ok(Branch::Implementation { issue: 7 })
        );
    }

    #[test]
    fn rejects_names_without_an_issue() {
        for name in ["feature-x", "-x", "12-", "12", "0-x", "1a-x", "", "x-12"] {
            assert!(Branch::parse(name).is_err(), "{name:?} should be rejected");
        }
    }
}
