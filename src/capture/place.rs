//! Where a session's main transcript is, so the cost of record can be read even
//! when the plugin's event that names the path never reached the store.

use std::env;
use std::path::PathBuf;

use rusqlite::Connection;

use super::rows;

/// The main transcript of `session_id`: the path the plugin sent, else the file
/// where Claude Code keeps the transcript of a session run in the folder of one
/// of the session's jobs, when that file exists. `None` when neither is known.
///
/// # Errors
/// A read of the store fails.
pub(crate) fn main_transcript(
    conn: &Connection,
    session_id: &str,
) -> rusqlite::Result<Option<String>> {
    if let Some(sent) = rows::session_transcript(conn, session_id)? {
        return Ok(Some(sent));
    }
    let Some(projects) = config_dir().map(|config| config.join("projects")) else {
        return Ok(None);
    };
    let found = rows::session_folders(conn, session_id)?
        .iter()
        .map(|folder| {
            projects
                .join(project_folder_name(folder))
                .join(format!("{session_id}.jsonl"))
        })
        .find(|path| path.is_file());
    Ok(found.and_then(|path| path.to_str().map(str::to_owned)))
}

/// Claude Code's config folder: `CLAUDE_CONFIG_DIR` when it is set, else
/// `.claude` in the home folder.
fn config_dir() -> Option<PathBuf> {
    env::var_os("CLAUDE_CONFIG_DIR")
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .or_else(|| env::home_dir().map(|home| home.join(".claude")))
}

/// The name of the folder below `projects` that holds the transcripts of the
/// sessions run in `folder`: each character that is not an ASCII letter or
/// digit becomes `-`, the slashes too.
fn project_folder_name(folder: &str) -> String {
    folder
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::project_folder_name;

    #[test]
    fn a_folder_is_named_with_each_mark_as_a_dash() {
        assert_eq!(
            project_folder_name("/Users/shaun/Code/super-rex.worktrees/v090/live t2"),
            "-Users-shaun-Code-super-rex-worktrees-v090-live-t2"
        );
        assert_eq!(
            project_folder_name("/private/tmp/claude-501/-Users-x"),
            "-private-tmp-claude-501--Users-x"
        );
    }
}
