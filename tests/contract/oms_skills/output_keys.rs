//! The skills copy the keys and lines of `rex` output into their text, so each
//! one they name must be in what the command really prints.

use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use serde_json::Value;
use super_rex::bundle::walk;
use super_rex::store::{Store, StoreError, StorePaths};

const DECIDE: &str = ".claude/skills/orchestration-builder/decide.md";
const RUN: &str = ".claude/skills/orchestration-builder/run.md";
const OPEN_JOB: &str = ".claude/skills/orchestration-builder/open-job.md";
const STAGE_1: &str = ".claude/skills/orchestration-builder/stages/1-skills/stage.md";
const STAGE_3: &str = ".claude/skills/orchestration-builder/stages/3-orchestration/stage.md";
const BRIEFS: &str = ".claude/skills/orchestration-builder/stages/3-orchestration/briefs.md";
const DELIVER: &str = ".claude/skills/rex-deliver/SKILL.md";
const FEEDBACK: &str = ".claude/skills/rex-feedback/SKILL.md";
const CAUSE: &str = ".claude/skills/rex-feedback/cause.md";

/// A plan with a capped role, a session role, and skills for two roles.
const PLAN: &str = r#"{
  "orchestrator": "pipeline",
  "skills": ["rex-code-tdd", "rex-code-philosophy"],
  "breakdown": "1. Reject zero.",
  "roles": [
    {"role": "engineer", "intelligence": "high", "reliability": "high", "consequence": "high", "verification": "low", "skills": ["rex-code-tdd", "rex-code-philosophy"]},
    {"role": "reviewer", "intelligence": "medium", "reliability": "medium", "consequence": "medium", "verification": "high", "skills": ["rex-code-philosophy"]},
    {"role": "decision-maker", "session": true}
  ]
}"#;

/// A repo with a store, two orchestrators, one skill, and a home folder.
struct World {
    _temp: tempfile::TempDir,
    paths: StorePaths,
    home: PathBuf,
}

impl World {
    fn new() -> World {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().canonicalize().unwrap();
        let repo = root.join("repo");
        fs::create_dir_all(repo.join(".git")).unwrap();
        let paths = StorePaths::locate(&repo).unwrap();
        drop(Store::create(&paths).unwrap());
        let skill = |folder: &Path, name: &str| {
            fs::create_dir_all(folder.join(name)).unwrap();
            fs::write(
                folder.join(name).join("SKILL.md"),
                format!("---\nname: {name}\ndescription: The {name} skill.\nwhen_to_use: Use when a test needs {name}.\n---\n\nBody.\n"),
            )
            .unwrap();
        };
        skill(&paths.orchestrators, "pipeline");
        skill(&paths.orchestrators, "spare");
        skill(&repo.join(".claude/skills"), "alpha");
        let home = root.join("home");
        fs::create_dir_all(&home).unwrap();
        World {
            _temp: temp,
            paths,
            home,
        }
    }

    /// The standard output of `rex <args>`, which must succeed.
    fn rex(&self, args: &[&str], stdin: &str) -> String {
        let output = Command::cargo_bin("rex")
            .unwrap()
            .current_dir(&self.paths.root)
            .env("HOME", &self.home)
            .env("CLAUDE_CODE_SESSION_ID", "contract-session")
            .env_remove("REX_STORE")
            .env_remove("CLAUDE_PROJECT_DIR")
            .env_remove("CLAUDE_EFFORT")
            .args(args)
            .write_stdin(stdin)
            .output()
            .unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "rex {args:?} failed: {stderr}");
        String::from_utf8(output.stdout).unwrap()
    }

    fn open_job(&self) -> String {
        let created = self.rex(
            &[
                "job",
                "create",
                "--request-file",
                "-",
                "--asked-by",
                "human",
                "--merge",
                "yes",
                "--type",
                "fix",
                "--size",
                "medium",
                "--consequence",
                "high",
                "--lang",
                "rust",
                "--budget",
                "low",
            ],
            "Reject a quantity of zero.\n",
        );
        value(&created, "job").to_owned()
    }

    /// One engineer run that opened one of its two skills.
    fn add_run(&self, job: &str) {
        Store::open(&self.paths)
            .unwrap()
            .write(|tx| {
                tx.execute(
                    "INSERT INTO sub_agent_run (job_id, session_id, agent_id, depth, role,
                                                skills_opened, first_seen_at, started_at, stopped_at)
                     VALUES (?1, 'contract-session', 'a1b8', 1, 'engineer', 'rex-code-tdd',
                             strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                             strftime('%Y-%m-%dT%H:%M:%fZ', 'now'),
                             strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
                    [job],
                )
                .map_err(StoreError::from)
            })
            .unwrap();
    }

    fn json(&self, args: &[&str]) -> Value {
        serde_json::from_str(&self.rex(args, "")).unwrap()
    }
}

fn value<'a>(stdout: &'a str, key: &str) -> &'a str {
    stdout
        .lines()
        .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
        .unwrap_or_else(|| panic!("no {key}= line in:\n{stdout}"))
}

fn skill_text(path: &str) -> String {
    let (_, bytes) = walk()
        .into_iter()
        .find(|(bundled, _)| bundled == Path::new(path))
        .unwrap_or_else(|| panic!("the bundle must hold {path}"));
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// True when `line` has the form of `template`, where each `<...>` stands for
/// any text.
fn fits(template: &str, line: &str) -> bool {
    let mut pieces = Vec::new();
    let mut rest = template;
    while let Some(open) = rest.find('<') {
        let close = rest[open..]
            .find('>')
            .map_or(rest.len(), |at| open + at + 1);
        pieces.push(&rest[..open]);
        rest = &rest[close..];
    }
    let Some((first, middle)) = pieces.split_first() else {
        return line == rest;
    };
    let Some(mut line) = line.strip_prefix(first) else {
        return false;
    };
    for piece in middle {
        let Some(at) = line.find(piece) else {
            return false;
        };
        line = &line[at + piece.len()..];
    }
    line.ends_with(rest)
}

/// The lines of the fenced block in `path` that starts with `start`.
fn block(path: &str, start: &str) -> Vec<String> {
    let text = skill_text(path);
    let found = text
        .split("```")
        .skip(1)
        .step_by(2)
        .map(|block| block.split_once('\n').map_or("", |(_, body)| body))
        .find(|body| body.starts_with(start))
        .unwrap_or_else(|| panic!("{path} must have a block that starts with {start:?}"));
    found.lines().map(str::to_owned).collect()
}

/// Each line of a block that fits no line of `output`.
fn unprinted(path: &str, block: &[String], output: &str) -> Vec<String> {
    block
        .iter()
        .filter(|template| !output.lines().any(|line| fits(template, line)))
        .map(|template| format!("{path}: `{template}` is not printed"))
        .collect()
}

/// The words in backticks after `anchor`, up to the end of its sentence,
/// leaving out the text in brackets, which tells what a value means.
fn keys_after(text: &str, anchor: &str) -> Vec<String> {
    let at = text
        .find(anchor)
        .unwrap_or_else(|| panic!("the skill text must hold {anchor:?}"));
    let rest = &text[at + anchor.len()..];
    let sentence = rest.lines().next().unwrap_or_default();
    let sentence = sentence.split(". ").next().unwrap_or_default();
    let mut plain = String::new();
    let mut depth = 0;
    for c in sentence.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            _ if depth == 0 => plain.push(c),
            _ => {}
        }
    }
    plain
        .split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

/// The keys of `rex job show`: each `key=` line and each `key:` field.
fn show_keys(output: &str) -> Vec<&str> {
    output
        .lines()
        .filter(|line| !line.starts_with(' '))
        .filter_map(|line| {
            line.split_once('=')
                .map(|(key, _)| key)
                .or(line.strip_suffix(':'))
        })
        .collect()
}

/// The JSON value at `path` of `record`: `job.orchestrator`, or `roles[]` for
/// the first item of `roles`.
fn at<'a>(record: &'a Value, path: &str) -> &'a Value {
    path.split('.')
        .fold(record, |value, part| match part.strip_suffix("[]") {
            Some(list) => &value[list][0],
            None => &value[part],
        })
}

/// The fields that a `jq` program reads: the list it opens with `.<list>[]`,
/// then each `.<key>` and each key of `{<key>, ...}`.
fn jq_fields(program: &str) -> (String, Vec<String>) {
    let mut fields = Vec::new();
    for (at, c) in program.char_indices() {
        let rest = &program[at + 1..];
        let end = |text: &str| text.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'));
        if c == '.' {
            let name = &rest[..end(rest).unwrap_or(rest.len())];
            if !name.is_empty() {
                fields.push(name.to_owned());
            }
        } else if c == '{' {
            let inside = &rest[..rest.find('}').unwrap_or(rest.len())];
            fields.extend(inside.split(',').map(|key| key.trim().to_owned()));
        }
    }
    let list = fields.remove(0);
    (list, fields)
}

// Eleven places in the skills copy what a `rex` command prints. An agent that
// reads a key the command no longer prints finds nothing, and goes on as if
// the value were empty.
#[test]
fn each_key_the_skills_copy_is_in_the_real_output() {
    let world = World::new();
    let mut gaps = Vec::new();

    // Job A runs and is rejected, so job B recalls its lesson and orchestrator.
    let job_a = world.open_job();
    let planned = world.rex(&["job", "plan", &job_a, "--plan-file", "-"], PLAN);
    gaps.extend(unprinted(
        DECIDE,
        &block(DECIDE, "job=<job ID>\nstatus=planned"),
        &planned,
    ));
    world.add_run(&job_a);
    let deliveries = r#"[{"repo": ".", "branch": "fix/zero", "merge_commit": "3f2a9c1e8b7d6a5f4e3d2c1b0a9f8e7d6c5b4a39", "outcome": "merged"}]"#;
    let closed = world.rex(
        &[
            "job",
            "close",
            &job_a,
            "--status",
            "delivered",
            "--outcome",
            "Zero is rejected",
            "--deliveries-file",
            "-",
        ],
        deliveries,
    );
    gaps.extend(unprinted(RUN, &block(RUN, "missed_skills=<n>"), &closed));
    let feedback = world.rex(
        &[
            "feedback",
            &job_a,
            "--verdict",
            "rejected",
            "--said",
            "Zero still passes.",
            "--cause",
            "agent-skill",
            "--lesson",
            "Test the edge.",
            "--evidence",
            "No test.",
        ],
        "",
    );
    let printed: Vec<String> = feedback
        .lines()
        .map(|line| format!("{}=", line.split('=').next().unwrap_or_default()))
        .collect();
    let listed = keys_after(&skill_text(FEEDBACK), "Output: four lines");
    if listed != printed {
        gaps.push(format!(
            "{FEEDBACK}: lists {listed:?}, `rex feedback` prints {printed:?}"
        ));
    }

    let job_b = world.open_job();
    let recalled = world.rex(&["recall", &job_b], "");
    gaps.extend(unprinted(
        DECIDE,
        &block(DECIDE, "job=<job ID>\nrecorded="),
        &recalled,
    ));

    let shown = world.rex(&["job", "show", &job_b], "");
    let keys = show_keys(&shown);
    for (path, anchor) in [
        (OPEN_JOB, "Take from output:"),
        (DELIVER, "rex job show \"$JOB\"`: keys"),
    ] {
        for key in keys_after(&skill_text(path), anchor) {
            if !keys.contains(&key.as_str()) {
                gaps.push(format!("{path}: `rex job show` prints no key `{key}`"));
            }
        }
    }

    let record = world.json(&["job", "show", &job_a, "--json"]);
    let feedback_text = skill_text(FEEDBACK);
    let read_from = &feedback_text[feedback_text.find("Read from it:").expect("Read from it:")..];
    let fields = read_from
        .lines()
        .skip(1)
        .take_while(|line| line.is_empty() || line.starts_with("- [ ]"))
        .filter_map(|line| line.strip_prefix("- [ ] `"));
    for line in fields {
        let Some((path, _)) = line.split_once("`: ") else {
            continue;
        };
        let object = at(&record, path);
        for key in keys_after(line, "`: ") {
            if object.get(&key).is_none() {
                gaps.push(format!(
                    "{FEEDBACK}: `{path}` of `rex job show --json` has no `{key}`"
                ));
            }
        }
    }

    let cause = skill_text(CAUSE);
    for (command, document) in [
        ("rex jobs --json | jq", world.json(&["jobs", "--json"])),
        ("rex job show \"$ID\" --json | jq", record.clone()),
    ] {
        let line = cause
            .lines()
            .find(|line| line.starts_with(command))
            .unwrap_or_else(|| panic!("{CAUSE} must run {command}"));
        let program = line.split('\'').nth(1).unwrap_or_default();
        let (list, keys) = jq_fields(program);
        for key in keys {
            if document[&list][0].get(&key).is_none() {
                gaps.push(format!("{CAUSE}: `{list}[]` of `{command}` has no `{key}`"));
            }
        }
    }

    let skills = world.rex(&["skills", "list"], "");
    gaps.extend(unprinted(STAGE_1, &block(STAGE_1, "name=<name>"), &skills));

    let oms = world.rex(&["oms", "--budget", "low", "--intelligence", "high"], "");
    let printed: Vec<&str> = oms
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, _)| key)
        .collect();
    let stage_3 = skill_text(STAGE_3);
    let table = &stage_3[stage_3
        .find("Output = six `key=value` lines")
        .expect("the oms table")..];
    let listed: Vec<String> = table
        .lines()
        .skip_while(|line| !line.starts_with('|'))
        .take_while(|line| line.starts_with('|'))
        .skip(2)
        .filter_map(|row| row.split('`').nth(1).map(str::to_owned))
        .collect();
    if listed != printed {
        gaps.push(format!(
            "{STAGE_3}: lists {listed:?}, `rex oms` prints {printed:?}"
        ));
    }

    let status = world.rex(&["status", &job_a], "");
    let header: Vec<&str> = status
        .lines()
        .next()
        .unwrap_or_default()
        .split_whitespace()
        .collect();
    for count in keys_after(
        &skill_text(BRIEFS),
        "header line for job: ID, status, counts",
    ) {
        if !header.contains(&count.as_str()) {
            gaps.push(format!(
                "{BRIEFS}: the header of `rex status` has no count `{count}`"
            ));
        }
    }

    assert!(
        gaps.is_empty(),
        "the skills copy output that `rex` does not print:\n{}",
        gaps.join("\n")
    );
}
