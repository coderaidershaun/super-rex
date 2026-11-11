//! A temporary repository with a real store, for tests that run the `rex`
//! binary against jobs they insert by SQL.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use rusqlite::Connection;
use super_rex::store::orchestrator;
use super_rex::store::{Store, StorePaths};
use tempfile::TempDir;

/// What one run of `rex` gave.
pub struct Run {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Run {
    pub fn stdout_lines(&self) -> Vec<&str> {
        self.stdout.lines().collect()
    }
}

/// A temporary repository, its store, and the means to fill and read it.
pub struct Fixture {
    _dir: TempDir,
    pub paths: StorePaths,
    conn: Connection,
    jobs_added: u32,
}

impl Fixture {
    pub fn new() -> Fixture {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();
        let paths = StorePaths::locate(dir.path()).unwrap();
        drop(Store::create(&paths).unwrap());
        let conn = Connection::open(&paths.db).unwrap();
        conn.execute_batch("PRAGMA foreign_keys = ON").unwrap();
        Fixture {
            _dir: dir,
            paths,
            conn,
            jobs_added: 0,
        }
    }

    /// Runs `rex` with the store of this fixture, from its repository.
    pub fn rex(&self, args: &[&str]) -> Run {
        let output = Command::cargo_bin("rex")
            .unwrap()
            .current_dir(&self.paths.root)
            .env("REX_STORE", &self.paths.dir)
            .env_remove("CLAUDE_PROJECT_DIR")
            .args(args)
            .output()
            .unwrap();
        Run {
            code: output.status.code().unwrap(),
            stdout: String::from_utf8(output.stdout).unwrap(),
            stderr: String::from_utf8(output.stderr).unwrap(),
        }
    }

    /// Starts a job. Nothing is stored until `insert`.
    pub fn add_job(&mut self, id: &str) -> JobBuilder<'_> {
        self.jobs_added += 1;
        let created_at = format!("2026-11-10T09:{:02}:00.000Z", self.jobs_added);
        JobBuilder {
            fixture: self,
            id: id.to_owned(),
            class: ["feature", "small", "low", "rust"].map(str::to_owned),
            status: "created".to_owned(),
            orchestrator: None,
            experience_recalled: None,
            created_at,
            request: "A request.".to_owned(),
        }
    }

    /// Makes `<orchestrators>/<name>/SKILL.md` and indexes it as version 1,
    /// the way a first `rex` run would, so a job can name it.
    pub fn orchestrator_that_ran(&self, name: &str, description: &str) {
        self.orchestrator_folder(name, description);
        let mut store = Store::open(&self.paths).unwrap();
        store
            .write(|tx| {
                let describe = |_: &Path| Some(description.to_owned());
                orchestrator::index(tx, &self.paths.orchestrators, name, &describe)
            })
            .unwrap();
    }

    /// Makes `<orchestrators>/<name>/SKILL.md` and leaves it out of the index.
    pub fn orchestrator_folder(&self, name: &str, description: &str) -> PathBuf {
        let folder = self.paths.orchestrators.join(name);
        fs::create_dir_all(&folder).unwrap();
        let file = folder.join("SKILL.md");
        fs::write(
            &file,
            format!("---\nname: {name}\ndescription: {description}\n---\n\nBody.\n"),
        )
        .unwrap();
        file
    }

    /// One column of one job, as text.
    pub fn job_column(&self, id: &str, column: &str) -> Option<String> {
        self.conn
            .query_row(
                &format!("SELECT {column} FROM job WHERE id = ?1"),
                [id],
                |row| row.get(0),
            )
            .unwrap()
    }

    pub fn status_of(&self, id: &str) -> String {
        self.job_column(id, "status").unwrap()
    }

    /// The number of rows a query counts.
    pub fn count(&self, sql: &str) -> i64 {
        self.conn.query_row(sql, [], |row| row.get(0)).unwrap()
    }

    /// A delivery of `job` that opened a pull request and waits for review.
    pub fn add_open_pr(&self, job: &str, repo: &str, branch: &str, pr: &str) {
        self.conn
            .execute(
                "INSERT INTO delivery (job_id, repo, branch, pr, merge_agreed, outcome, note,
                                       recorded_at)
                 VALUES (?1, ?2, ?3, ?4, 0, 'pr-open', 'Waiting for review.',
                         '2026-11-10T12:00:00.000Z')",
                (job, repo, branch, pr),
            )
            .unwrap();
    }

    /// The main session's cost of `job` in billionths of a dollar, with the
    /// requests it leaves out for want of a price, and its input, cache write
    /// and cache read tokens.
    pub fn add_session_tokens(&self, job: &str, cost: (i64, i64), tokens: [i64; 3]) {
        let (nanodollars, unpriced_requests) = cost;
        let [input, cache_creation, cache_read] = tokens;
        self.conn
            .execute(
                "INSERT INTO session_tokens (job_id, tokens_input, tokens_output,
                                             tokens_cache_read, tokens_cache_creation,
                                             cost_nanodollars, cost_unpriced_requests)
                 VALUES (?1, ?2, 0, ?3, ?4, ?5, ?6)",
                (
                    job,
                    input,
                    cache_read,
                    cache_creation,
                    nanodollars,
                    unpriced_requests,
                ),
            )
            .unwrap();
    }

    /// One run of `job`, with the same figures as `add_session_tokens`.
    pub fn add_run_tokens(&self, job: &str, agent: &str, nanodollars: i64, tokens: [i64; 3]) {
        let [input, cache_creation, cache_read] = tokens;
        self.conn
            .execute(
                "INSERT INTO sub_agent_run (job_id, session_id, agent_id, first_seen_at,
                                            tokens_input, tokens_output, tokens_cache_read,
                                            tokens_cache_creation, cost_nanodollars,
                                            cost_unpriced_requests)
                 VALUES (?1, 'session-' || ?1, ?2, '2026-11-10T10:00:00.000Z', ?3, 0, ?4, ?5,
                         ?6, 0)",
                (job, agent, input, cache_read, cache_creation, nanodollars),
            )
            .unwrap();
    }

    pub fn feedback_rows(&self, id: &str) -> i64 {
        self.count(&format!(
            "SELECT count(*) FROM feedback WHERE job_id = '{id}'"
        ))
    }
}

/// A job that is not stored yet.
pub struct JobBuilder<'a> {
    fixture: &'a Fixture,
    id: String,
    class: [String; 4],
    status: String,
    orchestrator: Option<(String, i64)>,
    experience_recalled: Option<String>,
    created_at: String,
    request: String,
}

impl JobBuilder<'_> {
    /// Work type, size, consequence and languages, as stored.
    pub fn class(
        mut self,
        work_type: &str,
        size: &str,
        consequence: &str,
        languages: &str,
    ) -> Self {
        self.class = [work_type, size, consequence, languages].map(str::to_owned);
        self
    }

    pub fn request(mut self, text: &str) -> Self {
        self.request = text.to_owned();
        self
    }

    pub fn created_at(mut self, timestamp: &str) -> Self {
        self.created_at = timestamp.to_owned();
        self
    }

    pub fn status(mut self, status: &str) -> Self {
        self.status = status.to_owned();
        self
    }

    /// The orchestrator must be in the index; see `orchestrator_that_ran`.
    pub fn orchestrator(mut self, name: &str, version: i64) -> Self {
        self.orchestrator = Some((name.to_owned(), version));
        self
    }

    pub fn experience_recalled(mut self, text: &str) -> Self {
        self.experience_recalled = Some(text.to_owned());
        self
    }

    /// Stores the job and gives its ID.
    pub fn insert(self) -> String {
        let [work_type, size, consequence, languages] = &self.class;
        let (name, version) = match &self.orchestrator {
            Some((name, version)) => (Some(name.as_str()), Some(*version)),
            None => (None, None),
        };
        self.fixture
            .conn
            .execute(
                "INSERT INTO job (id, request, asked_by, class_version, work_type, size,
                                  consequence, languages, merge_agreed, session_id,
                                  experience_recalled, orchestrator_name,
                                  orchestrator_version, status, created_at, updated_at)
                 VALUES (?1, ?12, 'human', 1, ?2, ?3, ?4, ?5, 0, ?6, ?7, ?8, ?9, ?10,
                         ?11, ?11)",
                rusqlite::params![
                    self.id,
                    work_type,
                    size,
                    consequence,
                    languages,
                    format!("session-{}", self.id),
                    self.experience_recalled,
                    name,
                    version,
                    self.status,
                    self.created_at,
                    self.request,
                ],
            )
            .unwrap();
        self.id
    }
}

/// The IDs of the jobs that the tests insert.
pub(super) const JOB_A: &str = "0g4aaaaaa1";
pub(super) const JOB_B: &str = "0g4bbbbbb2";
pub(super) const JOB_C: &str = "0g4cccccc3";
pub(super) const JOB_D: &str = "0g4dddddd4";
pub(super) const JOB_E: &str = "0g4eeeeee5";
pub(super) const JOB_F: &str = "0g4ffffff6";
pub(super) const JOB_G: &str = "0g4gggggg7";
pub(super) const JOB_H: &str = "0g4hhhhhh8";
