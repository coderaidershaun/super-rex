//! Reads an orchestrator's description from the header of its `SKILL.md`, for
//! the orchestrator index, which reads no headers itself.
//!
//! SMELL: this reads only the simplest header form, a one-line `description:`
//! value, and gives nothing for any other form. The skill commands have a full
//! header reader; use it here once it can be called from outside those commands.

use std::path::Path;

/// The `description:` of the header of `skill_file`: a one-line value, plain or
/// in quotes. `None` when the file, the header or the key is absent, or the
/// value takes another form, such as a block over several lines.
pub(super) fn description(skill_file: &Path) -> Option<String> {
    let text = std::fs::read_to_string(skill_file).ok()?;
    let lines: Vec<&str> = text.lines().collect();
    if lines.first()?.trim_end() != "---" {
        return None;
    }
    let end = 1 + lines[1..]
        .iter()
        .position(|line| line.trim_end() == "---")?;
    let header = &lines[1..end];
    let position = header
        .iter()
        .position(|line| line.starts_with("description:"))?;
    let continues_below = header
        .get(position + 1)
        .is_some_and(|next| next.starts_with([' ', '\t']));
    if continues_below {
        return None;
    }
    let value = header[position]["description:".len()..].trim();
    one_line_value(value)
}

/// A one-line YAML value: in double quotes with no escapes, in single quotes,
/// or plain. Anything harder gives `None`.
fn one_line_value(value: &str) -> Option<String> {
    if let Some(inner) = value
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
    {
        return (!inner.contains(['\\', '"'])).then(|| inner.to_owned());
    }
    if let Some(inner) = value
        .strip_prefix('\'')
        .and_then(|rest| rest.strip_suffix('\''))
    {
        return Some(inner.replace("''", "'"));
    }
    let plain = value.split(" #").next()?.trim_end();
    let starts_plain = plain
        .chars()
        .next()
        .is_some_and(|first| !"|>[]{}&*!%@`'\"#,?:-".contains(first));
    starts_plain.then(|| plain.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn description_of(text: &str) -> Option<String> {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("SKILL.md");
        std::fs::write(&file, text).unwrap();
        description(&file)
    }

    #[test]
    fn reads_one_line_descriptions_and_gives_none_for_the_rest() {
        let cases = [
            (
                "---\nname: a\ndescription: Plan, build.\n---\n",
                Some("Plan, build."),
            ),
            (
                "---\ndescription: \"Plan: build\"\n---\n",
                Some("Plan: build"),
            ),
            ("---\ndescription: 'It''s good'\n---\n", Some("It's good")),
            ("---\ndescription: Plan # a comment\n---\n", Some("Plan")),
            ("---\ndescription: >\n  Folded\n---\n", None),
            ("---\ndescription: Plan\n  and more\n---\n", None),
            ("---\nmeta:\n  description: Nested\n---\n", None),
            ("---\ndescription:\n---\n", None),
            ("---\ndescription: \"a \\\" b\"\n---\n", None),
            ("# No header\ndescription: Plan\n", None),
            ("---\ndescription: Never closed\n", None),
            ("---\nname: a\n---\ndescription: After the header\n", None),
        ];
        for (text, expected) in cases {
            assert_eq!(description_of(text).as_deref(), expected, "{text:?}");
        }
    }
}
