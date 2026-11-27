//! Reads libtest's plain-text output (`test path::name ... ok|FAILED|ignored`).

use std::collections::HashMap;

/// The outcome libtest reported for one test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Passed,
    Failed,
    Ignored,
}

/// Maps each test path in `output` to its outcome.
pub fn outcomes(output: &str) -> HashMap<String, Outcome> {
    output
        .lines()
        .filter_map(|line| {
            let (name, result) = line.strip_prefix("test ")?.split_once(" ... ")?;
            let outcome = match result.trim() {
                "ok" => Outcome::Passed,
                "FAILED" => Outcome::Failed,
                r if r.starts_with("ignored") => Outcome::Ignored,
                _ => return None,
            };
            Some((name.to_owned(), outcome))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Outcome, outcomes};

    #[test]
    fn reads_each_outcome() {
        let out = "\nrunning 4 tests\n\
                   test clients::rejects_bad_uri ... FAILED\n\
                   test clients::accepts_good_uri ... ok\n\
                   test slow ... ignored, takes a minute\n\
                   test top_level ... FAILED\n\
                   \nfailures:\n\n---- clients::rejects_bad_uri stdout ----\n\
                   test result: FAILED. 1 passed; 2 failed; 1 ignored\n";
        let got = outcomes(out);
        assert_eq!(got.len(), 4);
        assert_eq!(got["clients::rejects_bad_uri"], Outcome::Failed);
        assert_eq!(got["clients::accepts_good_uri"], Outcome::Passed);
        assert_eq!(got["slow"], Outcome::Ignored);
        assert_eq!(got["top_level"], Outcome::Failed);
    }
}
