//! The mod's tools that the skill text names: each tool name, and each argument
//! or result key the text names of a tool, is in the shared tools table, which
//! a kit test of the plugin pins to the tools it registers and their answers.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::LazyLock;

use regex::Regex;
use serde_json::Value;
use super_rex::bundle::{LIBRARY_ROOT, SKILLS_ROOT, walk};

const TABLE: &str = "fixtures/shared/tables/tools.json";

/// The tools of the shared table: for each one, its arguments and result keys
/// together, and for each tool a later unit adds, that unit.
struct ToolTable {
    names: BTreeMap<String, BTreeSet<String>>,
    later: BTreeMap<String, String>,
}

impl ToolTable {
    fn read() -> ToolTable {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(TABLE);
        let text = fs::read_to_string(&path).unwrap_or_else(|_| panic!("read {TABLE}"));
        let table: Value =
            serde_json::from_str(&text).unwrap_or_else(|_| panic!("{TABLE} is JSON"));
        let strings = |value: &Value| -> Vec<String> {
            value
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        };
        let names = table["tools"]
            .as_object()
            .unwrap_or_else(|| panic!("{TABLE} has a tools object"))
            .iter()
            .map(|(tool, entry)| {
                let mut names: BTreeSet<String> =
                    strings(&entry["arguments"]).into_iter().collect();
                names.extend(strings(&entry["keys"]));
                (tool.clone(), names)
            })
            .collect();
        let later = table["later"]
            .as_object()
            .into_iter()
            .flatten()
            .map(|(tool, unit)| (tool.clone(), unit.as_str().unwrap_or_default().to_owned()))
            .collect();
        ToolTable { names, later }
    }

    /// What is wrong with `name` as an argument or result key of `tool`, if
    /// anything.
    fn name_problem(&self, tool: &str, name: &str) -> Option<String> {
        if let Some(unit) = self.later.get(tool) {
            return Some(format!(
                "`{name}` of `{tool}`: {unit} adds {tool} to the mod later, so no key of it can be checked yet"
            ));
        }
        let names = self.names.get(tool)?;
        if names.iter().any(|listed| names_the_same(listed, name)) {
            return None;
        }
        Some(format!(
            "`{name}` of `{tool}` is no argument or result key of {tool} in {TABLE}"
        ))
    }
}

/// True when `written` names `listed`: the same text, where a `<placeholder>`
/// of `listed` stands for any text, and one of `written` for one of `listed`.
fn names_the_same(listed: &str, written: &str) -> bool {
    static PLACEHOLDER: LazyLock<Regex> =
        LazyLock::new(|| Regex::new("<[^>]+>").expect("placeholder pattern"));
    let pattern = PLACEHOLDER
        .split(listed)
        .map(regex::escape)
        .collect::<Vec<_>>()
        .join(".+");
    let any_text = Regex::new(&format!("^{pattern}$")).expect("pattern of a listed key");
    any_text.is_match(written)
        || PLACEHOLDER.replace_all(listed, "<>") == PLACEHOLDER.replace_all(written, "<>")
}

/// A tool name in skill text, short (`rex_merge`) or full
/// (`mcp__super-rex__rex_merge`). A name with a placeholder, such as
/// `rex_<name>`, is a template.
static TOOL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:^|[^A-Za-z0-9_-])(?:mcp__super-rex__)?(rex_[a-z][a-z0-9_]*)")
        .expect("tool pattern")
});

/// Code spans joined as a list: "`a`, `b` and `c`".
const SPANS: &str = r"`[^`]+`(?:(?:, and |, or |, | \+ | and | or )`[^`]+`)*";

/// The three ways skill text names an argument or a result key of a tool:
/// "`job`, `base` of `rex_job_open`", "`rex_merge` gives `merged=yes`" and
/// "`rex_status` with `job`". Each pattern captures the tool and the spans.
static NAMED: LazyLock<[(Regex, usize, usize); 3]> = LazyLock::new(|| {
    let tool = r"`(rex_[a-z][a-z0-9_]*)`";
    [
        (Regex::new(&format!("({SPANS}) of {tool}")), 2, 1),
        (
            Regex::new(&format!("{tool} (?:gives|gave|answers) ({SPANS})")),
            1,
            2,
        ),
        (Regex::new(&format!("{tool} with ({SPANS})")), 1, 2),
    ]
    .map(|(pattern, tool_group, spans_group)| {
        (pattern.expect("named pattern"), tool_group, spans_group)
    })
});

/// The argument or key a code span names: `merged` of `merged=yes`, `amend`
/// of `amend: true`. `None` for a span that is not a name, such as `[]`.
fn name_of(span: &str) -> Option<&str> {
    static NAME: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"^([a-z_][a-z0-9_.<>-]*)(?:=.*|: .*)?$").expect("name pattern")
    });
    NAME.captures(span)
        .and_then(|captures| captures.get(1))
        .map(|name| name.as_str())
}

/// Each tool name and each argument or key of a tool in `text` that the table
/// lacks, as `<path>:<line>: <what is wrong>`.
fn problems_in(path: &Path, text: &str, table: &ToolTable) -> Vec<String> {
    let mut problems = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let at = format!("{}:{}", path.display(), index + 1);
        for captures in TOOL.captures_iter(line) {
            let tool = &captures[1];
            if !table.names.contains_key(tool) && !table.later.contains_key(tool) {
                problems.push(format!("{at}: `{tool}` is no tool of the mod in {TABLE}"));
            }
        }
        for (pattern, tool_group, spans_group) in NAMED.iter() {
            for captures in pattern.captures_iter(line) {
                let tool = &captures[*tool_group];
                let spans = &captures[*spans_group];
                for span in spans.split('`').skip(1).step_by(2) {
                    let problem = name_of(span).and_then(|name| table.name_problem(tool, name));
                    problems.extend(problem.map(|problem| format!("{at}: {problem}")));
                }
            }
        }
    }
    problems
}

/// Each Markdown file of `plugin/skills` and `plugin/library`, as the plugin
/// ships it, with its text.
fn skill_text() -> Vec<(PathBuf, String)> {
    walk()
        .into_iter()
        .filter(|(path, _)| {
            [SKILLS_ROOT, LIBRARY_ROOT]
                .iter()
                .any(|root| path.starts_with(root))
                && path.extension().is_some_and(|extension| extension == "md")
        })
        .map(|(path, bytes)| (path, String::from_utf8_lossy(bytes).into_owned()))
        .collect()
}

// A skill that names a tool the mod does not have, or a key its tool never
// answers, sends the agent to a call that is refused or to a line that is not
// there, and the agent goes on without it.
#[test]
fn each_tool_and_key_the_skills_name_is_in_the_mod_table() {
    let table = ToolTable::read();
    let files = skill_text();
    assert!(
        files.iter().any(|(_, text)| TOOL.is_match(text)),
        "the skill text must name the mod's tools"
    );

    let problems: Vec<String> = files
        .iter()
        .flat_map(|(path, text)| problems_in(path, text, &table))
        .collect();
    assert!(
        problems.is_empty(),
        "the skills name tools or keys that the mod's table lacks:\n{}",
        problems.join("\n")
    );
}

#[test]
fn a_wrong_tool_or_key_in_a_copy_of_a_skill_is_named_with_its_file_and_line() {
    let table = ToolTable::read();
    let path = Path::new(SKILLS_ROOT).join("rex-deliver/SKILL.md");
    let (_, text) = skill_text()
        .into_iter()
        .find(|(file, _)| *file == path)
        .expect("the plugin ships rex-deliver/SKILL.md");
    let line_of = |words: &str| {
        let index = text
            .lines()
            .position(|line| line.contains(words))
            .unwrap_or_else(|| panic!("rex-deliver/SKILL.md holds {words}"));
        format!("{}:{}", path.display(), index + 1)
    };
    let (key_at, tool_at) = (
        line_of("`base` of `rex_job_open`"),
        line_of("`rex_merge` merges only"),
    );
    let wrong = text
        .replace("`base` of `rex_job_open`", "`bsae` of `rex_job_open`")
        .replace("`rex_merge` merges only", "`rex_merj` merges only");

    let mut problems = problems_in(&path, &wrong, &table);
    problems.sort();

    let mut expected = [
        format!(
            "{key_at}: `bsae` of `rex_job_open` is no argument or result key of rex_job_open in {TABLE}"
        ),
        format!("{tool_at}: `rex_merj` is no tool of the mod in {TABLE}"),
    ];
    expected.sort();
    assert_eq!(problems, expected);
}
