//! One open job per session: a second create in the session gives back the
//! open job, even when several creates run at the same moment.

use std::io::Write;
use std::process::{Child, Stdio};

use super::fixture::{
    PLAN, SESSION, create_args, new_repo, new_store, open_job, plan, rex, rex_process, show_json,
    stderr_of, value,
};

#[test]
fn create_in_session_with_open_job_returns_it() {
    let temp = tempfile::tempdir().unwrap();
    let main = new_repo(temp.path(), "shop");
    let store = new_store(&main, "pipeline");
    let job = open_job(
        &main,
        SESSION,
        &["--merge", "yes", "--type", "fix", "--lang", "rust"],
    );
    plan(&main, &job, PLAN);

    // A sub-agent's builder in the same session asks for a job of its own.
    let nested = rex(
        &main,
        &create_args(&["--merge", "no", "--type", "chore", "--lang", "none"]),
    )
    .env("CLAUDE_CODE_SESSION_ID", SESSION)
    .write_stdin("A sub-task.\n")
    .output()
    .unwrap();
    assert!(nested.status.success());
    assert_eq!(
        String::from_utf8(nested.stdout).unwrap(),
        format!("job={job}\nstatus=planned\nexisting=yes\n")
    );
    assert_eq!(
        String::from_utf8(nested.stderr).unwrap(),
        format!(
            "note: session {SESSION} already has open job {job}; no job was opened; this work \
             is recorded under it\n"
        )
    );

    let refused = stderr_of(
        rex(&main, &["job", "plan", &job, "--plan-file", "-"]).write_stdin(PLAN),
        1,
    );
    assert!(
        refused.contains("rex job plan accepts only a created job"),
        "{refused}"
    );
    let shown = show_json(&main, &job);
    assert_eq!(shown["transitions"].as_array().unwrap().len(), 2);
    assert_eq!(shown["roles"].as_array().unwrap().len(), 2);

    // Several creates at the same moment in a session with no open job: the
    // check and the insert are one write, so exactly one opens the job.
    let args = create_args(&["--merge", "no", "--type", "chore", "--lang", "none"]);
    let children: Vec<Child> = (0..8)
        .map(|_| {
            let mut child = rex_process(&main, &args)
                .env("CLAUDE_CODE_SESSION_ID", "parallel-session")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            child
                .stdin
                .take()
                .unwrap()
                .write_all(b"Parallel work.\n")
                .unwrap();
            child
        })
        .collect();
    let mut opened = Vec::new();
    let mut given = Vec::new();
    for child in children {
        let output = child.wait_with_output().unwrap();
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let job = value(&stdout, "job").to_owned();
        match value(&stdout, "existing") {
            "no" => opened.push(job),
            _ => given.push(job),
        }
    }
    assert_eq!(opened.len(), 1, "{opened:?}");
    assert!(given.iter().all(|job| *job == opened[0]), "{given:?}");
    let rows: i64 = store
        .conn()
        .query_row(
            "SELECT count(*) FROM job WHERE session_id = 'parallel-session'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(rows, 1);
}
