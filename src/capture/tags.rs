//! The tag block at the start of a brief: the job the starting agent thinks the
//! run works for, and the run's role. Hook events carry neither.

/// The known tags of one brief. Each is `None` when the brief has no such tag.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Tags {
    /// From `Rex-Job`.
    pub(crate) job: Option<String>,
    /// From `Rex-Role`.
    pub(crate) role: Option<String>,
}

impl Tags {
    /// The tags of `brief`. The block is the first lines that are tag lines, after
    /// any empty lines; the first line that is not a tag line ends it. A key given
    /// twice keeps its first value, and an unknown `Rex-` key is ignored, so keys can
    /// be added later without breaking this reader.
    pub(crate) fn of_brief(brief: &str) -> Tags {
        let mut tags = Tags::default();
        let lines = brief
            .split('\n')
            .map(|line| line.strip_suffix('\r').unwrap_or(line))
            .skip_while(|line| line.is_empty());
        for line in lines {
            let Some((key, value)) = tag_line(line) else {
                break;
            };
            let slot = match key {
                "Job" => &mut tags.job,
                "Role" => &mut tags.role,
                _ => continue,
            };
            if slot.is_none() {
                *slot = Some(value.to_owned());
            }
        }
        tags
    }
}

/// The key after `Rex-` and the trimmed value, IF `line` matches
/// `^Rex-[A-Za-z][A-Za-z-]*: \S.*$`.
fn tag_line(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix("Rex-")?;
    let key_end = rest
        .find(|c: char| !(c.is_ascii_alphabetic() || c == '-'))
        .unwrap_or(rest.len());
    let (key, rest) = rest.split_at_checked(key_end)?;
    if !key.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return None;
    }
    let value = rest.strip_prefix(": ")?;
    if !value.starts_with(|c: char| !c.is_whitespace()) {
        return None;
    }
    Some((key, value.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tags(job: Option<&str>, role: Option<&str>) -> Tags {
        Tags {
            job: job.map(str::to_owned),
            role: role.map(str::to_owned),
        }
    }

    #[test]
    fn normal_block() {
        let brief =
            "Rex-Job: k3x9p2m7qd\nRex-Role: engineer\n\nAdd the parser for the config file.\n";
        assert_eq!(
            Tags::of_brief(brief),
            tags(Some("k3x9p2m7qd"), Some("engineer"))
        );
    }

    #[test]
    fn order_reversed_with_an_unknown_key() {
        let brief = "Rex-Role: reviewer\nRex-Job: k3x9p2m7qd\nRex-Note: second pass\n\nRead the diff and report.\n";
        assert_eq!(
            Tags::of_brief(brief),
            tags(Some("k3x9p2m7qd"), Some("reviewer"))
        );
    }

    #[test]
    fn first_of_a_repeated_key_wins() {
        let brief = "Rex-Job: k3x9p2m7qd\nRex-Role: lead\nRex-Role: worker\n\nPlan the change, then start the workers.\n";
        assert_eq!(
            Tags::of_brief(brief),
            tags(Some("k3x9p2m7qd"), Some("lead"))
        );
    }

    #[test]
    fn no_block() {
        assert_eq!(
            Tags::of_brief("Find every caller of open_store.\n"),
            tags(None, None)
        );
    }

    #[test]
    fn block_not_at_the_start_is_not_a_block() {
        let brief = "Please do the task below.\nRex-Job: k3x9p2m7qd\nRex-Role: engineer\n";
        assert_eq!(Tags::of_brief(brief), tags(None, None));
    }

    #[test]
    fn empty_lines_first_carriage_returns_and_spaces_are_skipped() {
        let brief = "\n\r\nRex-Job: k3x9p2m7qd  \r\nRex-Role: code-lead\r\n\r\nGo.";
        assert_eq!(
            Tags::of_brief(brief),
            tags(Some("k3x9p2m7qd"), Some("code-lead"))
        );
    }

    #[test]
    fn a_line_that_does_not_match_ends_the_block() {
        let brief = "Rex-Job: k3x9p2m7qd\nrex-role: lead\nRex-Role: worker\n";
        assert_eq!(Tags::of_brief(brief), tags(Some("k3x9p2m7qd"), None));
        let brief = "Rex-Job:  k3x9p2m7qd\nRex-Role: lead\n";
        assert_eq!(Tags::of_brief(brief), tags(None, None));
        let brief = "Rex-1Job: x\nRex-Role: lead\n";
        assert_eq!(Tags::of_brief(brief), tags(None, None));
    }

    #[test]
    fn keys_are_case_sensitive() {
        let brief = "Rex-job: k3x9p2m7qd\nRex-Role: lead\n";
        assert_eq!(Tags::of_brief(brief), tags(None, Some("lead")));
    }
}
