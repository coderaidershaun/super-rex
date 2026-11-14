//! `rex tree show`: a master tree as the store holds it, with its jobs and its
//! questions, so a master that starts again picks up where the last one stopped.

use std::path::Path;

use rusqlite::{Connection, OptionalExtension, Row};
use serde_json::{Value, json};

use super::job::{JobError, QUESTION_COLUMNS, json_job, json_question, read_job, read_question};
use super::limit::SpendLimit;
use super::stdout::print_to_stdout;
use crate::store::status::Shown;
use crate::store::{JobId, Store, StorePaths};

/// Show a master tree.
#[derive(clap::Args)]
pub struct TreeOptions {
    #[command(subcommand)]
    pub command: TreeCommand,
}

/// The commands of `rex tree`.
#[derive(clap::Subcommand)]
pub enum TreeCommand {
    /// Print a master tree with its jobs and its questions.
    Show(ShowOptions),
}

/// Arguments of `rex tree show`.
#[derive(clap::Args)]
pub struct ShowOptions {
    /// The tree to show [default: the newest tree of the repo that is not closed].
    tree: Option<String>,

    /// Print one JSON line: the tree (or null), its jobs and its questions.
    #[arg(long)]
    json: bool,
}

/// # Errors
/// [`JobError::Store`] if the store cannot be found or read.
pub fn run(cwd: &Path, options: TreeOptions) -> Result<(), JobError> {
    let TreeCommand::Show(options) = options.command;
    let store = Store::open(&StorePaths::discover(cwd)?)?;
    let conn = store.conn();
    let tree = match &options.tree {
        Some(id) => conn
            .query_row(
                &format!("SELECT {TREE_COLUMNS} FROM tree WHERE id = ?1"),
                [id],
                tree_json,
            )
            .optional()?,
        None => conn
            .query_row(
                &format!(
                    "SELECT {TREE_COLUMNS} FROM tree WHERE closed_at IS NULL
                     ORDER BY started_at DESC, id DESC LIMIT 1"
                ),
                [],
                tree_json,
            )
            .optional()?,
    };
    let tree_id = tree
        .as_ref()
        .and_then(|tree| tree["id"].as_str())
        .map(str::to_owned);
    let (jobs, questions) = match &tree_id {
        Some(id) => (tree_jobs(&store, id)?, tree_questions(conn, id)?),
        None => (Vec::new(), Vec::new()),
    };
    let text = if options.json {
        format!(
            "{}\n",
            json!({ "tree": tree, "jobs": jobs, "questions": questions })
        )
    } else {
        format!(
            "tree={}\nstate={}\njobs={}\nquestions={}\n",
            tree_id.as_deref().unwrap_or("none"),
            tree.as_ref()
                .and_then(|tree| tree["state"].as_str())
                .unwrap_or("none"),
            jobs.len(),
            questions.len()
        )
    };
    print_to_stdout(&text)?;
    Ok(())
}

const TREE_COLUMNS: &str = "id, session_id, master_agent_id, charter, charter_hash, policy_hash, \
     state, cap_nano, report_file, started_at, closed_at";

fn tree_json(row: &Row<'_>) -> rusqlite::Result<Value> {
    let cap_nano: i64 = row.get(7)?;
    Ok(json!({
        "id": row.get::<_, String>(0)?,
        "session_id": row.get::<_, String>(1)?,
        "master_agent_id": row.get::<_, Option<String>>(2)?,
        "charter": row.get::<_, String>(3)?,
        "charter_hash": row.get::<_, String>(4)?,
        "policy_hash": row.get::<_, String>(5)?,
        "state": row.get::<_, String>(6)?,
        "cap_usd": cap_nano as f64 / 1_000_000_000.0,
        "report_file": row.get::<_, Option<String>>(8)?,
        "started_at": row.get::<_, String>(9)?,
        "closed_at": row.get::<_, Option<String>>(10)?,
    }))
}

/// The jobs of the tree, oldest first, each as `rex job show --json` prints its job.
fn tree_jobs(store: &Store, tree: &str) -> Result<Vec<Value>, JobError> {
    let ids: Vec<JobId> = {
        let mut statement = store
            .conn()
            .prepare("SELECT id FROM job WHERE tree_id = ?1 ORDER BY created_at, id")?;
        let rows = statement.query_map([tree], |row| row.get(0))?;
        rows.collect::<Result<_, _>>()?
    };
    let mut jobs = Vec::with_capacity(ids.len());
    for id in &ids {
        let record = read_job(store.conn(), store.paths(), id)?;
        let limit = SpendLimit::of(store, id)?;
        jobs.push(json_job(&record, Shown::of(record.status, 0), &limit));
    }
    Ok(jobs)
}

fn tree_questions(conn: &Connection, tree: &str) -> Result<Vec<Value>, JobError> {
    let mut statement = conn.prepare(&format!(
        "SELECT {QUESTION_COLUMNS} FROM question WHERE tree_id = ?1 ORDER BY asked_at, id"
    ))?;
    let rows = statement.query_map([tree], read_question)?;
    let questions = rows.collect::<Result<Vec<_>, _>>()?;
    Ok(questions.iter().map(json_question).collect())
}
