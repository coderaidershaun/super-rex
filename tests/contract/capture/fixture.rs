//! The recorded session of `tests/fixtures/capture/tree/`, set up in a temporary
//! repository with a store, so that a test chooses which log lines arrive and when.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use rusqlite::params;
use serde_json::Value;
use super_rex::capture::{CaptureError, JobRuns, Run, TokenCounts, job_runs};
use super_rex::store::{JobId, Store, StoreError, StorePaths};
use tempfile::TempDir;

pub(super) const SESSION: &str = "041bd529-f189-451f-9eaa-239f46bb7b36";
/// The job ID written in the recorded briefs.
pub(super) const JOB_A: &str = "k3x9p2m7qd";
pub(super) const JOB_B: &str = "b5n8w3r6tz";
/// Before the main session's first request, so a job opened then holds the whole session.
pub(super) const BEFORE_SESSION: &str = "2026-11-10T13:10:00.000Z";
/// One minute after the last recorded line.
pub(super) const AFTER_SESSION: &str = "2026-11-10T13:11:42.235Z";

pub(super) const BG: &str = "a70d8a0f630d8701c";
pub(super) const BG_CHILD: &str = "a0741c86685f0104e";
pub(super) const LEAD: &str = "acefc7f2b8d9b97bf";
pub(super) const WORKER: &str = "a8b103ab4ef48cdf5";
pub(super) const REVIEWER: &str = "ab99f22c637e9bce7";
pub(super) const PLAIN: &str = "a2caf7515d48b82f2";

const RECORDED_PROJECT: &str = "/home/fixture/.claude/projects/-home-fixture-work-repo";

/// The two variants of the tag block (key given twice; order reversed with an
/// unknown key), written into LEAD's and REVIEWER's briefs as JSON text.
const TAG_VARIANTS: [(&str, &str); 2] = [
    (
        r"Rex-Role: lead\n",
        r"Rex-Role: lead\nRex-Role: second-is-ignored\n",
    ),
    (
        r"Rex-Job: k3x9p2m7qd\nRex-Role: reviewer\n",
        r"Rex-Role: reviewer\nRex-Job: k3x9p2m7qd\nRex-Note: x\n",
    ),
];

/// A store in a temporary repository, the recorded transcripts beside it, and the
/// recorded log lines, which a test writes to the event log as the hook would.
pub(super) struct Fixture {
    pub(super) store: Store,
    /// In recorded order.
    pub(super) lines: Vec<String>,
    transcripts: PathBuf,
    _temp: TempDir,
}

impl Fixture {
    pub(super) fn new() -> Fixture {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let repo = root.join("repo");
        fs::create_dir_all(repo.join(".git")).unwrap();
        let store = Store::create(&StorePaths::locate(&repo).unwrap()).unwrap();

        let recorded = recorded_folder();
        let transcripts = root.join("transcripts");
        let edit = |text: String| {
            let mut text = text.replace(RECORDED_PROJECT, transcripts.to_str().unwrap());
            for (from, to) in TAG_VARIANTS {
                text = text.replace(from, to);
            }
            text
        };
        copy_folder(&recorded.join("transcripts"), &transcripts, &edit);
        let log = edit(fs::read_to_string(recorded.join("events.jsonl")).unwrap());
        for (_, variant) in TAG_VARIANTS {
            assert!(log.contains(variant), "variant {variant} not written");
        }
        Fixture {
            store,
            lines: log.lines().map(str::to_owned).collect(),
            transcripts,
            _temp: temp,
        }
    }

    /// The three recorded lines of `agent`'s run, in recorded order: its start, its
    /// stop, and the call that started it (for BG, its launch comes first).
    pub(super) fn lines_of(&self, agent: &str) -> Vec<String> {
        self.lines
            .iter()
            .filter(|line| {
                let event = &event_of(line);
                let is_its_own =
                    event["agent_id"] == agent && event["hook_event_name"] != "PostToolUse";
                is_its_own || event["tool_response"]["agentId"] == agent
            })
            .cloned()
            .collect()
    }

    /// Appends `lines` to the event log, as `rex hook` does.
    pub(super) fn append<S: AsRef<str>>(&self, lines: &[S]) {
        let mut log = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&self.store.paths().event_log)
            .unwrap();
        for line in lines {
            log.write_all(format!("{}\n", line.as_ref()).as_bytes())
                .unwrap();
        }
    }

    /// Opens a job of the recorded session, as `rex job create` does, at `created_at`.
    pub(super) fn open_job(&mut self, job: &str, created_at: &str) -> Result<(), StoreError> {
        self.store.write(|tx| {
            tx.execute(
                "INSERT INTO job (id, request, asked_by, class_version, work_type, size, consequence,
                                  languages, merge_agreed, session_id, status, created_at, updated_at)
                 VALUES (?1, 'record a session', 'human', 1, 'feature', 'small', 'low', 'rust', 0,
                         ?2, 'created', ?3, ?3)",
                params![job, SESSION, created_at],
            )?;
            tx.execute(
                "INSERT INTO job_transition (job_id, from_status, to_status, event, at)
                 VALUES (?1, NULL, 'created', 'job create', ?2)",
                params![job, created_at],
            )?;
            Ok(())
        })
    }

    /// Closes an open job at exactly `at`, as `rex job close` would.
    pub(super) fn close_job(&mut self, job: &str, at: &str) {
        self.store
            .write(|tx| {
                tx.execute(
                    "INSERT INTO job_transition (job_id, from_status, to_status, event, at)
                     SELECT id, status, 'cancelled', 'job close', ?2 FROM job WHERE id = ?1",
                    params![job, at],
                )?;
                tx.execute(
                    "UPDATE job SET status = 'cancelled', updated_at = ?2 WHERE id = ?1",
                    params![job, at],
                )?;
                Ok::<_, StoreError>(())
            })
            .unwrap();
    }

    /// The model and effort stored as the ones that ran the builder of `job`.
    pub(super) fn builder(&self, job: &str) -> (Option<String>, Option<String>) {
        self.store
            .conn()
            .query_row(
                "SELECT builder_model, builder_effort FROM job WHERE id = ?1",
                [job],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap()
    }

    /// The stored status of `job` and the number of its history rows.
    pub(super) fn status(&self, job: &str) -> (String, i64) {
        self.store
            .conn()
            .query_row(
                "SELECT status, (SELECT COUNT(*) FROM job_transition WHERE job_id = job.id)
                 FROM job WHERE id = ?1",
                [job],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap()
    }

    /// The runs of `job` as they stand at `now`, a time of the store's text form.
    pub(super) fn read(&mut self, job: &str, now: &str) -> JobRuns {
        self.try_read(job, now).unwrap()
    }

    pub(super) fn try_read(&mut self, job: &str, now: &str) -> Result<JobRuns, CaptureError> {
        let now = self.time(now);
        job_runs(&mut self.store, &job.parse::<JobId>().unwrap(), now)
    }

    /// The time that `text`, of the store's form, stands for.
    pub(super) fn time(&self, text: &str) -> SystemTime {
        let ms: i64 = self
            .store
            .conn()
            .query_row(
                "SELECT CAST(ROUND((julianday(?1) - 2440587.5) * 86400000) AS INTEGER)",
                [text],
                |row| row.get(0),
            )
            .unwrap();
        SystemTime::UNIX_EPOCH + Duration::from_millis(ms.try_into().unwrap())
    }

    pub(super) fn transcript(&self, agent: &str) -> PathBuf {
        self.subagents().join(format!("agent-{agent}.jsonl"))
    }

    pub(super) fn metadata(&self, agent: &str) -> PathBuf {
        self.subagents().join(format!("agent-{agent}.meta.json"))
    }

    pub(super) fn main_transcript(&self) -> PathBuf {
        self.transcripts.join(format!("{SESSION}.jsonl"))
    }

    fn subagents(&self) -> PathBuf {
        self.transcripts.join(SESSION).join("subagents")
    }
}

/// The hook event that a log line holds.
pub(super) fn event_of(line: &str) -> Value {
    serde_json::from_str::<Value>(line).unwrap()["event"].take()
}

/// Removes the last `count` lines of the file at `path`, and returns them.
pub(super) fn cut_last_lines(path: &Path, count: usize) -> String {
    let text = fs::read_to_string(path).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    let (kept, cut) = lines.split_at(lines.len() - count);
    fs::write(path, format!("{}\n", kept.join("\n"))).unwrap();
    format!("{}\n", cut.join("\n"))
}

/// Appends `text` to the file at `path`, as Claude Code adds records to a transcript.
pub(super) fn append_text(path: &Path, text: &str) {
    let mut file = OpenOptions::new().append(true).open(path).unwrap();
    file.write_all(text.as_bytes()).unwrap();
}

/// The run of `agent` in `runs`.
pub(super) fn run<'a>(runs: &'a JobRuns, agent: &str) -> &'a Run {
    runs.runs
        .iter()
        .find(|run| run.agent_id == agent)
        .unwrap_or_else(|| panic!("no run {agent} in {:?}", agent_ids(runs)))
}

pub(super) fn agent_ids(runs: &JobRuns) -> Vec<&str> {
    runs.runs.iter().map(|run| run.agent_id.as_str()).collect()
}

/// `tree/expected.json`: what the recorded session holds, worked out by a script
/// that shares no code with the crate.
pub(super) fn expected() -> Value {
    let text = fs::read_to_string(recorded_folder().join("expected.json")).unwrap();
    serde_json::from_str(&text).unwrap()
}

/// The sum over the models of one `tokens` object of `expected.json`.
pub(super) fn expected_tokens(by_model: &Value) -> TokenCounts {
    let mut total = TokenCounts::default();
    for counts in by_model.as_object().unwrap().values() {
        let count = |key: &str| counts[key].as_u64().unwrap();
        total += TokenCounts {
            input: count("input"),
            output: count("output"),
            cache_read: count("cache_read"),
            cache_creation: count("cache_write"),
        };
    }
    total
}

fn recorded_folder() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/capture/tree")
}

fn copy_folder(from: &Path, to: &Path, edit: &dyn Fn(String) -> String) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_folder(&entry.path(), &target, edit);
        } else {
            let text = fs::read_to_string(entry.path()).unwrap();
            fs::write(target, edit(text)).unwrap();
        }
    }
}
