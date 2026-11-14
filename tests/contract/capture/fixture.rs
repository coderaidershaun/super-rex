//! The recorded session of `tests/fixtures/capture/tree/`, set up in a temporary
//! repository with a store, so that a test chooses which of its runs the plugin
//! reports through `rex ingest`, and when.

use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use rusqlite::params;
use serde_json::{Value, json};
use super_rex::capture::{CaptureError, JobRuns, Run, TokenCounts, job_runs};
use super_rex::ingest::ingest;
use super_rex::store::{JobId, Store, StoreError, StorePaths};
use tempfile::TempDir;

pub(super) const SESSION: &str = "041bd529-f189-451f-9eaa-239f46bb7b36";
/// The job ID written in the recorded briefs.
pub(super) const JOB_A: &str = "k3x9p2m7qd";
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
/// recorded hook lines of rex 0.7, which a test turns into the events the plugin
/// sends for the same session.
pub(super) struct Fixture {
    pub(super) store: Store,
    /// In recorded order.
    pub(super) lines: Vec<String>,
    transcripts: PathBuf,
    /// The last sequence number sent for the session.
    sequence: u64,
    /// The job the plugin gives the runs it reports: the newest one opened.
    job: Option<String>,
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
            sequence: 0,
            job: None,
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

    /// Sends the events the plugin sends for `lines` through `rex ingest`: the
    /// session's start once, a run's start for each start line and its end for
    /// each stop line. The parent, the depth and the role of a run are what the
    /// plugin knows at its spawn, taken here from the recorded call that started it.
    pub(super) fn append<S: AsRef<str>>(&mut self, lines: &[S]) {
        let spawns = spawns(&self.lines);
        let mut events = Vec::new();
        for line in lines {
            let recorded: Value = serde_json::from_str(line.as_ref()).unwrap();
            let event = &recorded["event"];
            let at = self.text_of_ms(recorded["at_ms"].as_u64().unwrap());
            if self.sequence == 0 && events.is_empty() {
                let data = json!({ "claude_code": "2.1.323", "plugin": "0.8.0", "storage": "on",
                                   "transcript_path": event["transcript_path"] });
                events.push(plugin_event("session.start", &at, None, data));
            }
            let agent = event["agent_id"].as_str().unwrap_or_default();
            match event["hook_event_name"].as_str() {
                Some("SubagentStart") => {
                    let spawn = spawns.get(agent);
                    let data = json!({
                        "agent_id": agent,
                        "parent": spawn.map_or("main", |spawn| spawn.parent.as_str()),
                        "parent_source": "engine",
                        "depth": spawn.map_or(1, |spawn| spawn.depth),
                        "agent_type": event["agent_type"],
                        "planned": null, "role": spawn.and_then(|spawn| spawn.role.clone()),
                        "task": null, "tool_use_id": null, "brief_hash": null,
                        "assembled": "no", "lint": null, "skills_attached": [], "tree": null
                    });
                    events.push(plugin_event("run.start", &at, self.job.as_deref(), data));
                }
                Some("SubagentStop") => {
                    let mut data = json!({
                        "agent_id": agent, "reason": "answer",
                        "tokens": { "input": 0, "output": 0, "cache_read": 0, "cache_creation": 0 },
                        "spent_nano": 0,
                        "final_message": event["last_assistant_message"].as_str().unwrap_or_default(),
                    });
                    if let Some(path) = event["agent_transcript_path"].as_str() {
                        data["transcript_path"] = json!(path);
                    }
                    events.push(plugin_event("run.end", &at, self.job.as_deref(), data));
                }
                _ => {}
            }
        }
        self.send(events);
    }

    /// Sends `events` as one batch, numbered after the last one sent.
    pub(super) fn send(&mut self, mut events: Vec<Value>) {
        for event in &mut events {
            self.sequence += 1;
            event["seq"] = json!(self.sequence);
        }
        let batch = json!({ "version": 1, "session": SESSION, "stream": "aaaaaaaaaaaaaaaa",
                            "secret": "1".repeat(64), "events": events });
        ingest(&mut self.store, batch.to_string().as_bytes(), &|_| None).unwrap();
    }

    /// `at_ms` as the store writes a time.
    fn text_of_ms(&self, at_ms: u64) -> String {
        self.store
            .conn()
            .query_row(
                "SELECT strftime('%Y-%m-%dT%H:%M:%fZ', ?1 / 1000.0, 'unixepoch')",
                [at_ms as i64],
                |row| row.get(0),
            )
            .unwrap()
    }

    /// Opens a job of the recorded session at `created_at`; the runs reported
    /// after it are the job's.
    pub(super) fn open_job(&mut self, job: &str, created_at: &str) -> Result<(), StoreError> {
        self.job = Some(job.to_owned());
        self.store.write(|tx| {
            tx.execute(
                "INSERT INTO job (id, request, origin, class_version, work_type, size, consequence,
                                  languages, merge_agreed, session_id, status, created_at, updated_at)
                 VALUES (?1, 'record a session', 'composer', 1, 'feature', 'small', 'low', 'rust', 0,
                         ?2, 'created', ?3, ?3)",
                params![job, SESSION, created_at],
            )?;
            tx.execute(
                "INSERT INTO job_transition (job_id, from_status, to_status, event, at)
                 VALUES (?1, NULL, 'created', 'job open', ?2)",
                params![job, created_at],
            )?;
            Ok(())
        })
    }

    /// Closes an open job at exactly `at`, as the plugin's close of it would.
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

/// What the plugin knows of a run when it starts.
struct Spawn {
    parent: String,
    depth: u64,
    role: Option<String>,
}

/// The spawn of each run of `lines`: the agent whose call of the Agent tool
/// started it (`main` for the main session), its depth, and the role its brief names.
fn spawns(lines: &[String]) -> HashMap<String, Spawn> {
    let mut parents = HashMap::new();
    for line in lines {
        let event = event_of(line);
        let Some(child) = event["tool_response"]["agentId"].as_str() else {
            continue;
        };
        let parent = event["agent_id"].as_str().unwrap_or("main").to_owned();
        let role = event["tool_input"]["prompt"]
            .as_str()
            .and_then(|brief| {
                brief
                    .lines()
                    .find_map(|line| line.strip_prefix("Rex-Role: "))
            })
            .map(str::to_owned);
        parents.insert(child.to_owned(), (parent, role));
    }
    let depth_of = |agent: &str| {
        let mut depth = 0;
        let mut current = agent.to_owned();
        while let Some((parent, _)) = parents.get(&current) {
            depth += 1;
            current = parent.clone();
        }
        depth.max(1)
    };
    parents
        .iter()
        .map(|(child, (parent, role))| {
            let spawn = Spawn {
                parent: parent.clone(),
                depth: depth_of(child),
                role: role.clone(),
            };
            (child.clone(), spawn)
        })
        .collect()
}

/// One event of the plugin, for [`Fixture::send`] to number.
fn plugin_event(kind: &str, at: &str, job: Option<&str>, data: Value) -> Value {
    json!({ "at": at, "kind": kind, "agent": "main", "job": job, "data": data })
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
