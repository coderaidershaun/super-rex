//! Each session's main transcript, kept in the store's `meta` table from the
//! session's start event, and the time of one write.

use rusqlite::{Connection, OptionalExtension, params};

const TRANSCRIPT_KEY: &str = "intake.transcript.";

/// The current time as the store writes it, read once for one write.
pub(crate) fn now_text(conn: &Connection) -> rusqlite::Result<String> {
    conn.query_row("SELECT strftime('%Y-%m-%dT%H:%M:%fZ', 'now')", [], |row| {
        row.get(0)
    })
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
