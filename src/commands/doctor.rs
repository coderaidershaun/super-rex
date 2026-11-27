//! `rex doctor`: whether this repo's binary, store, config, plugin and Claude
//! Code fit together, and which open jobs no live session owns any more.

use std::ffi::OsString;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

use rusqlite::{Connection, OptionalExtension};
use serde_json::{Value, json};

use super::stdout::print_to_stdout;
use crate::error::{CommandError, FailureClass, RexError};
use crate::store::{Store, StoreError, StorePaths};

/// The longest doctor waits for the library server to answer `head`, so a
/// server that hangs never keeps doctor from ending.
const SERVER_TIME_LIMIT: Duration = Duration::from_secs(10);

/// The key of the library server under `mcpServers` in the plugin manifest.
const SERVER_KEY: &str = "library";

/// What Claude Code puts the plugin's folder in place of, in a server entry.
const PLUGIN_ROOT: &str = "${CLAUDE_PLUGIN_ROOT}";

/// The id of the `tools/call` of `head` that doctor sends.
const HEAD_ID: u64 = 2;

/// Check that the binary, the store, the config, the plugin and Claude Code fit.
#[derive(clap::Args)]
#[command(
    after_help = "Prints `doctor=ok` or `doctor=problems`, then `binary=`, `store=`, `storage=`, `claude=`, `plugin=` (the versions the newest session reported), `library=` (the name and version the library server answers, or `unreachable`), and one `problem=` line for each thing to fix. Exits 0 either way."
)]
pub struct DoctorOptions {
    /// A plugin folder: start the library server its manifest declares, as Claude Code does, and compare the manifest's version with the binary's. Without it, doctor serves the embedded library with this binary.
    #[arg(long, value_name = "DIR")]
    pub plugin: Option<PathBuf>,
}

#[derive(thiserror::Error, Debug)]
pub enum DoctorError {
    #[error(transparent)]
    Store(#[from] StoreError),
    #[error(transparent)]
    Rex(#[from] RexError),
}

impl CommandError for DoctorError {
    fn failure_class(&self) -> FailureClass {
        match self {
            DoctorError::Store(error) => error.failure_class(),
            DoctorError::Rex(error) => error.failure_class(),
        }
    }

    fn exit_code(&self) -> u8 {
        match self {
            DoctorError::Store(error) => error.exit_code(),
            DoctorError::Rex(error) => error.exit_code(),
        }
    }
}

/// What doctor found, one field for each line it prints before the problems.
struct Findings {
    store: String,
    storage: String,
    claude: String,
    plugin: String,
    library: String,
    problems: Vec<String>,
}

/// # Errors
/// [`StoreError::NotInGitRepo`] when `cwd` is in no Git repository; a store
/// that cannot be read is a problem line, not an error.
pub fn run(cwd: &Path, options: DoctorOptions) -> Result<(), DoctorError> {
    let root = StorePaths::locate(cwd)?.root;
    let mut findings = Findings {
        store: "none".to_owned(),
        storage: "missing".to_owned(),
        claude: "unknown".to_owned(),
        plugin: "unknown".to_owned(),
        library: "unreachable".to_owned(),
        problems: Vec::new(),
    };
    let config = findings.read_config(&root);
    findings.read_store(cwd, config.as_ref());
    let (server, manifest_version) = match &options.plugin {
        None => (own_server(), None),
        Some(folder) => declared_server(&cwd.join(folder)),
    };
    let binary = env!("CARGO_PKG_VERSION");
    let stored_version = Some(findings.plugin.clone()).filter(|version| version != "unknown");
    let mut skewed: Vec<String> = Vec::new();
    for version in [stored_version, manifest_version].into_iter().flatten() {
        if version != binary && !skewed.contains(&version) {
            skewed.push(version);
        }
    }
    for version in skewed {
        findings.problems.push(format!(
            "version skew: plugin {version}, binary {binary}; install the same version of both"
        ));
    }
    if let Some(minimum) = config
        .as_ref()
        .and_then(|config| config.get("min_claude_code"))
        .and_then(Value::as_str)
        && is_older(&findings.claude, minimum)
    {
        findings.problems.push(format!(
            "Claude Code {} is older than {minimum}, the oldest the plugin runs on; run claude update",
            findings.claude
        ));
    }
    match server.and_then(|server| server.ask_head(SERVER_TIME_LIMIT)) {
        Ok(head) => findings.library = head,
        Err(cause) => findings.problems.push(format!("library server: {cause}")),
    }

    let mut text = format!(
        "doctor={}\nbinary={binary}\nstore={}\nstorage={}\nclaude={}\nplugin={}\nlibrary={}\n",
        if findings.problems.is_empty() {
            "ok"
        } else {
            "problems"
        },
        findings.store,
        findings.storage,
        findings.claude,
        findings.plugin,
        findings.library,
    );
    for problem in &findings.problems {
        text.push_str(&format!("problem={}\n", one_line(problem)));
    }
    print_to_stdout(&text)?;
    Ok(())
}

impl Findings {
    /// The config of the repo, or `None` with a problem when it is missing or unreadable.
    fn read_config(&mut self, root: &Path) -> Option<Value> {
        let path = root.join(".rex/config.json");
        let text = match fs::read(&path) {
            Ok(text) => text,
            Err(_) => {
                self.problems.push(
                    "no .rex/config.json, so the plugin is off in this repo; run rex init"
                        .to_owned(),
                );
                return None;
            }
        };
        match serde_json::from_slice::<Value>(&text) {
            Ok(config) => {
                let storage = config
                    .get("storage")
                    .and_then(Value::as_str)
                    .unwrap_or("off");
                self.storage = storage.to_owned();
                Some(config)
            }
            Err(error) => {
                self.storage = "unreadable".to_owned();
                self.problems.push(format!(
                    ".rex/config.json is not valid JSON ({error}), so the plugin is off in this repo; correct it or run rex init"
                ));
                None
            }
        }
    }

    /// The store's version, the versions the newest session reported, and the
    /// open jobs of older sessions.
    fn read_store(&mut self, cwd: &Path, config: Option<&Value>) {
        let is_storage_on = config
            .and_then(|config| config.get("storage"))
            .and_then(Value::as_str)
            == Some("on");
        let store = match StorePaths::discover(cwd).and_then(|paths| Store::open(&paths)) {
            Ok(store) => store,
            Err(StoreError::NotFound { .. }) => {
                if is_storage_on {
                    self.problems.push(
                        "storage is on but there is no store; run rex init --with-storage"
                            .to_owned(),
                    );
                }
                return;
            }
            Err(error @ StoreError::VersionOne { .. }) => {
                self.store = "v1".to_owned();
                self.problems.push(error.to_string());
                return;
            }
            Err(StoreError::NewerSchema { found, .. }) => {
                self.store = format!("v{found}");
                self.problems.push(format!(
                    "store version {found} is newer than this rex; install a newer rex"
                ));
                return;
            }
            Err(error) => {
                self.store = "unreadable".to_owned();
                self.problems
                    .push(format!("the store cannot be opened: {error}"));
                return;
            }
        };
        self.store = "v2".to_owned();
        if let Err(error) = self.read_record(store.conn()) {
            self.problems
                .push(format!("the store cannot be read: {error}"));
        }
    }

    fn read_record(&mut self, conn: &Connection) -> rusqlite::Result<()> {
        let newest_start: Option<String> = conn
            .query_row(
                "SELECT data FROM ingest_event WHERE kind = 'session.start'
                 ORDER BY at DESC, seq DESC LIMIT 1",
                [],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(data) = newest_start.and_then(|data| serde_json::from_str::<Value>(&data).ok())
        {
            let text = |key: &str| data.get(key).and_then(Value::as_str).map(str::to_owned);
            self.claude = text("claude_code").unwrap_or_else(|| "unknown".to_owned());
            self.plugin = text("plugin").unwrap_or_else(|| "unknown".to_owned());
        }
        for (job, status, session) in orphans(conn)? {
            self.problems.push(format!(
                "open orphan: job {job} is {status}, but its session {session} is not the newest; close it, or resume its tree with /super-rex:orchestration-master resume"
            ));
        }
        Ok(())
    }
}

/// The open jobs whose session is not the newest one that wrote to the store:
/// a session ended without closing them. Newest is the session of the newest
/// event, or of the newest job when no event came.
fn orphans(conn: &Connection) -> rusqlite::Result<Vec<(String, String, String)>> {
    let mut statement = conn.prepare(
        "WITH newest AS (
             SELECT session_id FROM (
                 SELECT session_id, at FROM ingest_event
                 UNION ALL SELECT session_id, created_at FROM job)
             ORDER BY at DESC LIMIT 1)
         SELECT id, status, session_id FROM job
         WHERE status IN ('created', 'planned')
           AND session_id <> (SELECT session_id FROM newest)
         ORDER BY created_at, id",
    )?;
    let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
    rows.collect()
}

/// True when `found` is a version older than `minimum`, both `major.minor.patch`.
fn is_older(found: &str, minimum: &str) -> bool {
    let parts = |text: &str| -> Option<Vec<u32>> {
        text.split('.').map(|part| part.parse().ok()).collect()
    };
    match (parts(found), parts(minimum)) {
        (Some(found), Some(minimum)) => found < minimum,
        _ => false,
    }
}

/// A problem on one line, whatever text an error brought into it.
fn one_line(text: &str) -> String {
    text.replace("\r\n", " ").replace(['\r', '\n'], " ")
}

/// A command that starts a library server, as a plugin manifest declares it.
struct ServerCommand {
    program: OsString,
    args: Vec<OsString>,
    /// The plugin folder, which Claude Code also gives the server as `CLAUDE_PLUGIN_ROOT`.
    plugin_root: Option<PathBuf>,
}

/// This binary as `rex skills serve`, over the library embedded in it.
fn own_server() -> Result<ServerCommand, String> {
    let program = std::env::current_exe()
        .map_err(|error| format!("cannot find this rex binary to start it: {error}"))?;
    Ok(ServerCommand {
        program: program.into_os_string(),
        args: vec!["skills".into(), "serve".into()],
        plugin_root: None,
    })
}

/// The library server that `<folder>/.claude-plugin/plugin.json` declares,
/// with the folder in place of `${CLAUDE_PLUGIN_ROOT}`, and the manifest's
/// `version` when it has one.
fn declared_server(folder: &Path) -> (Result<ServerCommand, String>, Option<String>) {
    let path = folder.join(".claude-plugin/plugin.json");
    let manifest = fs::read(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))
        .and_then(|bytes| {
            serde_json::from_slice::<Value>(&bytes)
                .map_err(|error| format!("{} is not valid JSON: {error}", path.display()))
        });
    let manifest = match manifest {
        Ok(manifest) => manifest,
        Err(cause) => return (Err(cause), None),
    };
    let version = manifest
        .get("version")
        .and_then(Value::as_str)
        .map(str::to_owned);
    let root = folder.to_string_lossy();
    let in_place = |text: &str| OsString::from(text.replace(PLUGIN_ROOT, &root));
    let entry = &manifest["mcpServers"][SERVER_KEY];
    let Some(command) = entry.get("command").and_then(Value::as_str) else {
        let cause = format!(
            "{} declares no command for mcpServers.{SERVER_KEY}",
            path.display()
        );
        return (Err(cause), version);
    };
    let args: Option<Vec<OsString>> = match entry.get("args") {
        None => Some(Vec::new()),
        Some(args) => args
            .as_array()
            .and_then(|args| args.iter().map(|arg| arg.as_str().map(in_place)).collect()),
    };
    let Some(args) = args else {
        let cause = format!(
            "the args of mcpServers.{SERVER_KEY} in {} are not a list of text",
            path.display()
        );
        return (Err(cause), version);
    };
    let server = ServerCommand {
        program: in_place(command),
        args,
        plugin_root: Some(folder.to_path_buf()),
    };
    (Ok(server), version)
}

impl ServerCommand {
    /// Starts the server, sends `initialize` and a `tools/call` of `head`, and
    /// gives `<library> <version>` from its answer. Waits at most `limit`.
    fn ask_head(&self, limit: Duration) -> Result<String, String> {
        let mut command = Command::new(&self.program);
        command
            .args(&self.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(root) = &self.plugin_root {
            command.env("CLAUDE_PLUGIN_ROOT", root);
        }
        let mut child = command
            .spawn()
            .map_err(|error| format!("cannot start {}: {error}", self.program.to_string_lossy()))?;
        let answer = talk(&mut child, limit);
        stop(&mut child);
        head_line(&answer?)
    }
}

/// The answer to `head`, or why none came within `limit`. Stdin stays open
/// until the answer comes, as it does for a server that Claude Code starts.
fn talk(child: &mut Child, limit: Duration) -> Result<Value, String> {
    let deadline = Instant::now() + limit;
    let mut stdin = child.stdin.take().expect("doctor pipes the server's stdin");
    let requests = [
        json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
            "protocolVersion": "2025-06-18", "capabilities": {},
            "clientInfo": { "name": "rex-doctor", "version": env!("CARGO_PKG_VERSION") } } }),
        json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }),
        json!({ "jsonrpc": "2.0", "id": HEAD_ID, "method": "tools/call",
                "params": { "name": "head", "arguments": {} } }),
    ];
    for request in &requests {
        // A server that already ended refuses the write; what it wrote on
        // stderr is the cause doctor reports, so the write error adds nothing.
        if writeln!(stdin, "{request}").is_err() {
            break;
        }
    }

    let stdout = child
        .stdout
        .take()
        .expect("doctor pipes the server's stdout");
    let (answer_sender, answers) = mpsc::channel();
    thread::spawn(move || answer_sender.send(head_answer(stdout)));
    let mut stderr = child
        .stderr
        .take()
        .expect("doctor pipes the server's stderr");
    let (error_sender, errors) = mpsc::channel();
    thread::spawn(move || {
        let mut text = String::new();
        let read = stderr.read_to_string(&mut text);
        error_sender.send(read.map(|_| text).unwrap_or_default())
    });

    match answers.recv_timeout(limit) {
        Ok(Some(answer)) => Ok(answer),
        Err(RecvTimeoutError::Timeout) => Err(format!("no answer to head within {limit:?}")),
        _ => {
            let left = deadline.saturating_duration_since(Instant::now());
            let stderr = errors.recv_timeout(left).unwrap_or_default();
            Err(match stderr.lines().find(|line| !line.trim().is_empty()) {
                Some(line) => format!("it ended with no answer to head: {}", line.trim()),
                None => "it ended with no answer to head and wrote nothing on stderr".to_owned(),
            })
        }
    }
}

/// The JSON-RPC answer whose id is the `head` call's, or `None` when stdout
/// ends first. Lines that are not JSON are skipped.
fn head_answer(stdout: ChildStdout) -> Option<Value> {
    BufReader::new(stdout)
        .lines()
        .map_while(Result::ok)
        .filter_map(|line| serde_json::from_str::<Value>(&line).ok())
        .find(|answer| answer["id"] == HEAD_ID)
}

/// `<library> <version>` from the answer to `head`.
fn head_line(answer: &Value) -> Result<String, String> {
    if let Some(error) = answer.get("error") {
        return Err(format!("it refused tools/call: {error}"));
    }
    let result = &answer["result"];
    let text = result["content"][0]["text"]
        .as_str()
        .ok_or_else(|| format!("its answer to head holds no text: {answer}"))?;
    let head: Value = serde_json::from_str(text)
        .map_err(|error| format!("its answer to head is not JSON ({error}): {text}"))?;
    if result["isError"] == true {
        return Err(format!("head failed: {head}"));
    }
    match (head["library"].as_str(), head["version"].as_str()) {
        (Some(library), Some(version)) => Ok(format!("{library} {version}")),
        _ => Err(format!(
            "its answer to head has no library or version: {head}"
        )),
    }
}

/// Ends the server. A server that already ended cannot be killed, and an
/// ended server is what doctor wants, so neither result changes anything.
fn stop(child: &mut Child) {
    child.kill().ok();
    child.wait().ok();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_server_that_never_answers_is_given_up_at_the_time_limit() {
        let server = ServerCommand {
            program: "sleep".into(),
            args: vec!["30".into()],
            plugin_root: None,
        };
        let started = Instant::now();

        let cause = server.ask_head(Duration::from_millis(300)).unwrap_err();

        assert_eq!(cause, "no answer to head within 300ms");
        assert!(started.elapsed() < Duration::from_secs(5));
    }
}
