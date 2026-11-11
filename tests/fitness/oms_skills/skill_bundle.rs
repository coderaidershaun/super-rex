//! The skills, library skills and agents embedded in the binary are checked with
//! the same rules as `rex skills check`, so that `rex init` never installs a broken one.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super_rex::bundle::{AGENTS_ROOT, LIBRARY_ROOT, SKILLS_ROOT, walk};
use super_rex::commands::skills::{KnownSkills, Rule, Skill, SkillFile, check_skill};

/// One skill of the bundle, with the root folder that holds it, such as
/// `.rex/skills`.
struct BundledSkill {
    root: &'static str,
    skill: Skill,
}

/// Each skill folder of the bundle with its Markdown files, in
/// `.claude/skills` and in the library, and the skills and Markdown files
/// around them. The bundle is Markdown, so a Markdown file that is not UTF-8
/// is a mistake.
fn bundle_skills() -> (Vec<BundledSkill>, KnownSkills) {
    let mut files_by_folder: BTreeMap<(&str, String), Vec<SkillFile>> = BTreeMap::new();
    let mut known = KnownSkills::default();

    for (path, bytes) in walk() {
        let Some(root) = [SKILLS_ROOT, LIBRARY_ROOT]
            .into_iter()
            .find(|root| path.starts_with(root))
        else {
            continue;
        };
        let below_root = path.strip_prefix(root).expect("the path is below its root");
        if below_root.extension().is_none_or(|ext| ext != "md") {
            continue;
        }
        let text = String::from_utf8(bytes.to_vec())
            .unwrap_or_else(|_| panic!("{} is not UTF-8", path.display()));
        let in_root = if root == LIBRARY_ROOT {
            &mut known.library_paths
        } else {
            &mut known.skill_paths
        };
        in_root.insert(below_root.to_path_buf());

        let mut parts = below_root.components();
        let Some(folder) = parts.next() else {
            continue;
        };
        let in_skill: PathBuf = parts.as_path().to_path_buf();
        if in_skill.as_os_str().is_empty() {
            continue;
        }
        let folder = folder.as_os_str().to_string_lossy().into_owned();
        files_by_folder
            .entry((root, folder))
            .or_default()
            .push(SkillFile {
                path: in_skill,
                text,
            });
    }

    let skills = files_by_folder
        .into_iter()
        .map(|((root, folder), files)| BundledSkill {
            root,
            skill: Skill { folder, files },
        })
        .collect();
    (skills, known)
}

/// What is around a skill in `root`: every skill of both folders, and the
/// Markdown files of `root`, where a link may point.
fn known_from(root: &str, known: &KnownSkills) -> KnownSkills {
    let files = if root == LIBRARY_ROOT {
        &known.library_paths
    } else {
        &known.skill_paths
    };
    KnownSkills {
        files: files.clone(),
        ..known.clone()
    }
}

/// The most words a stage file of the builder may hold, counted as `wc -w`
/// counts them. The decision agent reads each stage file in full, like a
/// `SKILL.md`, so it has the same limit.
const MAX_STAGE_WORDS: usize = 1500;

/// The stage files of the builder: stages 1 to 3 and the challenge.
const STAGE_FILES: usize = 4;

/// Each stage file of the builder, `stages/<stage>/stage.md`, that holds more
/// than `MAX_STAGE_WORDS` words, as `<skill>/<file>: <count> words`. Panics
/// unless it finds all `STAGE_FILES`, so a renamed folder cannot pass by
/// checking nothing.
fn long_stage_files(skills: &[BundledSkill]) -> Vec<String> {
    let stage_files: Vec<_> = skills
        .iter()
        .map(|bundled| &bundled.skill)
        .filter(|skill| skill.folder == "orchestration-builder")
        .flat_map(|skill| skill.files.iter().map(move |file| (skill, file)))
        .filter(|(_, file)| {
            file.path.starts_with("stages") && file.path.file_name() == Some("stage.md".as_ref())
        })
        .collect();
    assert_eq!(
        stage_files.len(),
        STAGE_FILES,
        "orchestration-builder must have {STAGE_FILES} files stages/<stage>/stage.md"
    );
    stage_files
        .into_iter()
        .filter_map(|(skill, file)| {
            let words = file.text.split_whitespace().count();
            (words > MAX_STAGE_WORDS).then(|| {
                format!(
                    "{}/{}: {words} words, more than {MAX_STAGE_WORDS}",
                    skill.folder,
                    file.path.display()
                )
            })
        })
        .collect()
}

/// Each bundled agent whose header does not take the Skill tool away, by path.
/// The bundle must hold at least one agent.
fn agents_with_the_skill_tool() -> Vec<String> {
    let agents: Vec<(PathBuf, String)> = walk()
        .into_iter()
        .filter(|(path, _)| path.starts_with(AGENTS_ROOT))
        .map(|(path, bytes)| {
            let text = String::from_utf8(bytes.to_vec())
                .unwrap_or_else(|_| panic!("{} is not UTF-8", path.display()));
            (path, text)
        })
        .collect();
    assert!(!agents.is_empty(), "the bundle must hold agents");

    agents
        .into_iter()
        .filter(|(_, text)| {
            let mut header = text.lines().skip(1).take_while(|line| *line != "---");
            !header.any(|line| line == "disallowedTools: Skill")
        })
        .map(|(path, _)| format!("{}: no `disallowedTools: Skill`", path.display()))
        .collect()
}

/// Each finding of one of `rules`, as `<root>/<skill>/<file>: <rule>: <message>`.
fn findings_of(skills: &[BundledSkill], known: &KnownSkills, rules: &[Rule]) -> Vec<String> {
    skills
        .iter()
        .flat_map(|BundledSkill { root, skill }| {
            check_skill(skill, &known_from(root, known))
                .into_iter()
                .filter(|finding| rules.contains(&finding.rule))
                .map(move |finding| {
                    let file = Path::new(root).join(&skill.folder).join(&finding.file);
                    format!(
                        "{}: {}: {}",
                        file.display(),
                        finding.rule.id(),
                        finding.message
                    )
                })
        })
        .collect()
}

// This test must never be deleted. A skill whose name does not match its
// folder, whose description is missing, or whose header or text breaks
// another rule is skipped or cut short by Claude Code, and nothing reports it.
// A skill or a stage file of the builder over a word limit fills the context
// of every agent that reads it. An agent that keeps the Skill tool can load
// skills its brief never gave it. It also fails on an empty bundle, which
// would pass every other check.
#[test]
fn every_skill_and_agent_of_the_bundle_obeys_its_rules() {
    let (skills, known) = bundle_skills();
    for root in [SKILLS_ROOT, LIBRARY_ROOT] {
        assert!(
            skills.iter().any(|bundled| bundled.root == root),
            "the bundle must hold skills in {root}"
        );
    }

    let rules = [
        Rule::H1,
        Rule::H2,
        Rule::H3,
        Rule::H4,
        Rule::H5,
        Rule::H7,
        Rule::H8,
        Rule::H9,
        Rule::B1,
        Rule::B2,
        Rule::B4,
        Rule::B5,
    ];
    let mut findings = findings_of(&skills, &known, &rules);
    findings.extend(long_stage_files(&skills));
    findings.extend(agents_with_the_skill_tool());
    assert!(
        findings.is_empty(),
        "the bundled skills and agents break these rules:\n{}",
        findings.join("\n")
    );
}

// This test must never be deleted. A space followed by `#` starts a comment in
// a plain YAML value, so `Inline #[cfg(test)]` cuts a description or a
// `when_to_use` down to its first words, and the skill stops loading when a
// person asks for it.
#[test]
fn a_plain_description_is_not_shortened_by_yaml() {
    let (skills, known) = bundle_skills();

    let findings = findings_of(&skills, &known, &[Rule::H6]);
    assert!(
        findings.is_empty(),
        "YAML cuts these header values short:\n{}",
        findings.join("\n")
    );
}

/// The assignment table: stage 1, stage 2, the challenger, `rex-rules` and
/// `rex-feedback` all start from its rows.
const ASSIGNMENT_TABLE: &str = ".claude/skills/orchestration-builder/stages/1-skills/assign.md";

/// The words of `text` in backticks.
fn backticked(text: &str) -> impl Iterator<Item = &str> {
    text.split('`').skip(1).step_by(2)
}

/// Each name in backticks in the assignment table that the bundle does not
/// hold: a skill `rex-<name>`, a sibling file `<skill>/<file>.md`, or a path
/// `<root>/<skill>/<file>.md`. A gap skill (`rex-gap-<language>`) is written
/// by a job, not bundled, and a name with a placeholder is a template.
fn dead_names_in_assignment_table(bundle: &BTreeMap<PathBuf, String>) -> Vec<String> {
    let roots = [SKILLS_ROOT, LIBRARY_ROOT];
    let is_bundled = |path: &str| bundle.contains_key(Path::new(path));
    let is_skill = |name: &str| {
        roots.iter().any(|root| {
            bundle
                .keys()
                .any(|path| path.starts_with(Path::new(root).join(name)))
        })
    };
    let text = bundle
        .get(Path::new(ASSIGNMENT_TABLE))
        .unwrap_or_else(|| panic!("the bundle must hold {ASSIGNMENT_TABLE}"));
    backticked(text)
        .filter(|name| !name.contains(['<', ' ']))
        .filter(|name| {
            if name.ends_with(".md") && name.contains('/') {
                if roots.iter().any(|root| name.starts_with(root)) {
                    !is_bundled(name)
                } else {
                    !roots
                        .iter()
                        .any(|root| is_bundled(&format!("{root}/{name}")))
                }
            } else {
                name.starts_with("rex-") && !name.starts_with("rex-gap-") && !is_skill(name)
            }
        })
        .map(|name| format!("{ASSIGNMENT_TABLE}: `{name}` is not in the bundle"))
        .collect()
}

/// Each "`## <Section>` of `<path>`" (or `in`) in the bundle whose file, given
/// by its full path, has no line `## <Section>`.
fn dead_section_references(bundle: &BTreeMap<PathBuf, String>) -> Vec<String> {
    let mut dead = Vec::new();
    for (from, text) in bundle {
        let parts: Vec<&str> = text.split('`').collect();
        for at in (1..parts.len().saturating_sub(2)).step_by(2) {
            let (section, between, target) = (parts[at], parts[at + 1], parts[at + 2]);
            let names_its_file = section.starts_with("## ")
                && matches!(between, " of " | " in ")
                && [SKILLS_ROOT, LIBRARY_ROOT]
                    .iter()
                    .any(|root| target.starts_with(root));
            if !names_its_file {
                continue;
            }
            let is_there = bundle
                .get(Path::new(target))
                .is_some_and(|file| file.lines().any(|line| line.trim_end() == section));
            if !is_there {
                dead.push(format!(
                    "{}: `{section}` is not a heading of {target}",
                    from.display()
                ));
            }
        }
    }
    dead
}

// The links, the assignment table and the section names are how one skill
// sends an agent to another file. A name that is not there sends every task
// that the row fits to a file it cannot read, and the agent goes on without it.
#[test]
fn every_reference_to_a_file_in_the_bundle_resolves() {
    let (skills, known) = bundle_skills();
    let bundle: BTreeMap<PathBuf, String> = walk()
        .into_iter()
        .map(|(path, bytes)| (path, String::from_utf8_lossy(bytes).into_owned()))
        .collect();

    let mut findings = findings_of(&skills, &known, &[Rule::B3]);
    findings.extend(dead_names_in_assignment_table(&bundle));
    findings.extend(dead_section_references(&bundle));
    assert!(
        findings.is_empty(),
        "these references point to nothing in the bundle:\n{}",
        findings.join("\n")
    );
}
