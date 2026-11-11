//! The builder skill tells an agent which classification values to give
//! `rex job create`, so its lists must hold exactly the values it accepts.

use std::collections::BTreeSet;

use clap::ValueEnum;
use super_rex::bundle::walk;
use super_rex::store::class::{Consequence, Size, WorkType};

/// The text of each Markdown file of the `orchestration-builder` skill.
fn builder_texts() -> Vec<String> {
    walk()
        .into_iter()
        .filter(|(path, _)| {
            path.starts_with(".claude/skills/orchestration-builder")
                && path.extension().is_some_and(|ext| ext == "md")
        })
        .map(|(path, bytes)| {
            String::from_utf8(bytes.to_vec())
                .unwrap_or_else(|_| panic!("{} is not UTF-8", path.display()))
        })
        .collect()
}

/// The words in backticks on the one line that starts with `label`. A list
/// mark `- ` before the label is allowed.
fn listed_values(texts: &[String], label: &str) -> BTreeSet<String> {
    let lines: Vec<&str> = texts
        .iter()
        .flat_map(|text| text.lines())
        .map(str::trim)
        .map(|line| line.strip_prefix("- ").unwrap_or(line))
        .filter(|line| line.starts_with(label))
        .collect();
    let [line] = lines.as_slice() else {
        panic!(
            "exactly one line of the builder skill must start with `{label}`, found {}",
            lines.len()
        );
    };

    line.split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

/// The values `rex job create` accepts for `T`, as typed on the command line.
fn accepted_values<T: ValueEnum>() -> BTreeSet<String> {
    T::value_variants()
        .iter()
        .filter_map(ValueEnum::to_possible_value)
        .map(|value| value.get_name().to_owned())
        .collect()
}

#[test]
fn the_builder_lists_exactly_the_classification_values() {
    let texts = builder_texts();
    let labels = [
        ("Work type values:", accepted_values::<WorkType>()),
        ("Size values:", accepted_values::<Size>()),
        ("Consequence values:", accepted_values::<Consequence>()),
    ];

    for (label, accepted) in labels {
        let listed = listed_values(&texts, label);
        let not_listed: Vec<&String> = accepted.difference(&listed).collect();
        let not_accepted: Vec<&String> = listed.difference(&accepted).collect();
        assert!(
            not_listed.is_empty() && not_accepted.is_empty(),
            "`{label}` leaves out {not_listed:?}, which `rex job create` accepts, \
             and lists {not_accepted:?}, which it does not"
        );
    }
}
