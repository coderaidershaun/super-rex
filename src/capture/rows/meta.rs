//! The intake's saved values in the store's `meta` table: how far the event log was
//! read, how many of its lines were damaged, and each session's main transcript.

use rusqlite::{Connection, OptionalExtension, params};

const OFFSET: &str = "intake.offset";
const DAMAGED_LINES: &str = "intake.damaged_lines";
const TRANSCRIPT_KEY: &str = "intake.transcript.";

/// The current time as the store writes it, read once for one write.
pub(crate) fn now_text(conn: &Connection) -> rusqlite::Result<String> {
    conn.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now')", [], |row| {
        row.get(0)
    })
}

/// How far into the event log intake has read, in bytes.
pub(crate) fn offset(conn: &Connection) -> rusqlite::Result<u64> {
    Ok(count(conn, OFFSET)?.unwrap_or(0))
}

/// Damaged lines of the whole log, so far.
pub(crate) fn damaged_lines(conn: &Connection) -> rusqlite::Result<u64> {
    Ok(count(conn, DAMAGED_LINES)?.unwrap_or(0))
}

/// Saves the new offset, and adds the damaged lines of this intake to the count.
pub(crate) fn save_intake(conn: &Connection, offset: u64, damaged: u64) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![OFFSET, offset.to_string()],
    )?;
    if damaged > 0 {
        conn.execute(
            "INSERT INTO meta (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET
                 value = CAST(CAST(value AS INTEGER) + CAST(excluded.value AS INTEGER) AS TEXT)",
            params![DAMAGED_LINES, damaged.to_string()],
        )?;
    }
    Ok(())
}

/// Keeps the first main transcript path seen for a session.
pub(crate) fn remember_transcript(
    conn: &Connection,
    session_id: &str,
    path: &str,
) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT INTO meta (key, value) VALUES (?1 || ?2, ?3) ON CONFLICT(key) DO NOTHING",
        params![TRANSCRIPT_KEY, session_id, path],
    )?;
    Ok(())
}

/// The main transcript of a session that logged at least one line.
pub(crate) fn session_transcript(
    conn: &Connection,
    session_id: &str,
) -> rusqlite::Result<Option<String>> {
    conn.query_row(
        "SELECT value FROM meta WHERE key = ?1 || ?2",
        params![TRANSCRIPT_KEY, session_id],
        |row| row.get(0),
    )
    .optional()
}

fn count(conn: &Connection, key: &str) -> rusqlite::Result<Option<u64>> {
    let value: Option<String> = conn
        .query_row("SELECT value FROM meta WHERE key = ?1", [key], |row| {
            row.get(0)
        })
        .optional()?;
    Ok(value.and_then(|value| value.parse().ok()))
}
