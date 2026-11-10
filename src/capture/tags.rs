//! The tag block at the start of a brief: the job the starting agent thinks the
//! run works for, and the run's role. Hook events carry neither.

/// The known tags of one brief. Each is `None` when the brief has no such tag.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Tags {
    /// From `Rex-Job`. A cross-check only: it never moves a run to another job,
    /// because a lead may copy a wrong block. The tie comes from the session and
    /// the times, which Claude Code gives.
    pub(crate) job: Option<String>,
    /// From `Rex-Role`.
    pub(crate) role: Option<String>,
}

impl Tags {
    /// The tags of `brief`. The block is the first lines that are tag lines, after
    /// any lines that hold only spaces, tabs or `\r`; the first line that is not a
    /// tag line ends it. A key given twice keeps its first value, and an unknown
    /// `Rex-` key is ignored, so keys can be added later without breaking this reader.
    pub(crate) fn of_brief(brief: &str) -> Tags {
        let mut tags = Tags::default();
        let lines = brief
            .split('\n')
            .skip_while(|line| line.chars().all(|c| matches!(c, ' ' | '\t' | '\r')))
            .map(|line| line.strip_suffix('\r').unwrap_or(line));
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

/// The key after `Rex-` and the value trimmed of spaces and tabs, IF `line` matches
/// `^Rex-([A-Za-z][A-Za-z-]*):[ \t]*(\S.*)$`.
fn tag_line(line: &str) -> Option<(&str, &str)> {
    let rest = line.strip_prefix("Rex-")?;
    let key_end = rest
        .find(|c: char| !(c.is_ascii_alphabetic() || c == '-'))
        .unwrap_or(rest.len());
    let (key, rest) = rest.split_at_checked(key_end)?;
    if !key.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return None;
    }
    let value = rest.strip_prefix(':')?.trim_start_matches([' ', '\t']);
    if !value.starts_with(|c: char| !c.is_whitespace()) {
        return None;
    }
    Some((key, value.trim_end_matches([' ', '\t'])))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tag_block_rules() {
        let job = Some("k3x9p2m7qd".to_owned());
        let role = Some("lead".to_owned());
        let cases = [
            (
                "blank lines before the block",
                "\n \t\nRex-Job: k3x9p2m7qd\nRex-Role: lead\n",
                (job.clone(), role.clone()),
            ),
            (
                "carriage return at each line end",
                "\r\nRex-Job: k3x9p2m7qd \t\r\nRex-Role: lead\r\n\r\nGo.",
                (job.clone(), role.clone()),
            ),
            (
                "any spaces or tabs after the colon",
                "Rex-Job:k3x9p2m7qd\nRex-Role: \t lead\n",
                (job.clone(), role.clone()),
            ),
            (
                "a block that is not at the start",
                "Please do the task below.\nRex-Job: k3x9p2m7qd\nRex-Role: lead\n",
                (None, None),
            ),
        ];
        for (name, brief, (job, role)) in cases {
            assert_eq!(Tags::of_brief(brief), Tags { job, role }, "{name}");
        }
    }
}
