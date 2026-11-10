//! The text that the job commands store and print for a command-line value. It
//! is the value's own name on the command line, so the two never differ.

use clap::ValueEnum;
use rusqlite::Row;
use rusqlite::types::Type;

use crate::oms::Level;

/// The name of `value` on the command line, such as `low` or `delivered`.
pub(super) fn value_name(value: impl ValueEnum) -> String {
    let name = value
        .to_possible_value()
        .expect("no value of a job command is skipped on the command line");
    name.get_name().to_owned()
}

/// A stored rating column: `low`, `medium`, `high`, or NULL for none.
pub(super) fn stored_level(row: &Row<'_>, index: usize) -> rusqlite::Result<Option<Level>> {
    let text: Option<String> = row.get(index)?;
    text.map(|text| {
        <Level as ValueEnum>::from_str(&text, false).map_err(|_| {
            rusqlite::Error::FromSqlConversionFailure(
                index,
                Type::Text,
                format!("{text} is not a rating; one of low, medium, high").into(),
            )
        })
    })
    .transpose()
}
