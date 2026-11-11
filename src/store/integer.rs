//! SQLite holds every integer as a signed 64-bit value, and rex counts bytes,
//! tokens and milliseconds as `u64`. These convert at the edge of each statement.

use rusqlite::Row;

/// The value to bind. No count or time of rex comes near `i64::MAX`.
pub(crate) fn signed(value: u64) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

pub(crate) fn signed_or_null(value: Option<u64>) -> Option<i64> {
    value.map(signed)
}

/// The column at `index` as a count. A negative value is an error, not a wrong count.
pub(crate) fn unsigned(row: &Row<'_>, index: usize) -> rusqlite::Result<Option<u64>> {
    let value: Option<i64> = row.get(index)?;
    value
        .map(|value| {
            u64::try_from(value).map_err(|_| rusqlite::Error::IntegralValueOutOfRange(index, value))
        })
        .transpose()
}
