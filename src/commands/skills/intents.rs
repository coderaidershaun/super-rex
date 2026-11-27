//! The intent check of `rex skills check`: each intent phrase of a checked skill must find that
//! skill near the top of a keyword search among the checked skills and a pool of other skills.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use super::header::Header;
use super::list;
use super::rules::{Finding, Rule, Skill};
use super::search::{Index, KeywordTable, SkillEntry, SkillSource};
use super::{SKILL_FILE, SkillsError};

/// Each intent phrase must find its own skill within this many places.
const INTENT_TOP: usize = 5;

/// The fewest skills that make the intent check a real test: the checked
/// skills alone are too few to compete with.
const MIN_INTENT_POOL: usize = 1000;

/// The library name of the checked skills in the index of the intent check.
const CHECKED_LIBRARY: &str = "checked";

const SKIP_LINE: &str = "warning: intent check skipped; give --pool <folder> of other skills \
    to check that each intent finds its skill";

/// Whether the header's `metadata` map holds the key `intents`, whatever
/// its value.
pub(super) fn has_intents(skill: &Skill) -> bool {
    Header::of_skill_file(skill_md(skill)).is_some_and(|header| {
        header
            .metadata()
            .is_some_and(|map| map.get("intents").is_some())
    })
}

/// Say on stderr that the intent check did not run because no pool was given.
pub(super) fn print_skip_line() {
    print_line_to_stderr(SKIP_LINE);
}

fn skill_md(skill: &Skill) -> &str {
    skill
        .files
        .iter()
        .find(|file| file.path == Path::new(SKILL_FILE))
        .map_or("", |file| file.text.as_str())
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned()
}

/// The index the intent check searches: each checked skill at the position
/// it has in `skills`, then each skill of each pool folder.
fn intent_index(cwd: &Path, skills: &[&Skill], pools: &[PathBuf]) -> Result<Index, SkillsError> {
    let mut entries: Vec<SkillEntry> = skills
        .iter()
        .map(|skill| {
            SkillEntry::read(SkillSource {
                name: skill.folder.clone(),
                library: CHECKED_LIBRARY.to_owned(),
                skill_md: skill_md(skill).to_owned(),
                ..SkillSource::default()
            })
        })
        .collect();
    for (position, pool) in pools.iter().enumerate() {
        entries.extend(pool_entries(cwd, pool, position + 1)?);
    }
    Ok(Index { skills: entries })
}

/// One entry for each skill folder of the pool folder `written`, in name
/// order. Pool skills are only searched among, so one whose `SKILL.md`
/// cannot be read is left out and the check goes on.
fn pool_entries(
    cwd: &Path,
    written: &Path,
    library_priority: usize,
) -> Result<Vec<SkillEntry>, SkillsError> {
    let folder = cwd.join(written);
    if let Err(error) = fs::metadata(&folder) {
        if error.kind() == io::ErrorKind::NotFound {
            return Err(SkillsError::Missing {
                path: written.to_path_buf(),
            });
        }
        return Err(SkillsError::Read {
            path: written.to_path_buf(),
            source: error,
        });
    }
    let mut folders = list::skill_folders(&folder)?;
    folders.sort();
    let library = file_name(&folder);
    let entries = folders
        .iter()
        .filter_map(|skill_folder| {
            let bytes = fs::read(skill_folder.join(SKILL_FILE)).ok()?;
            Some(SkillEntry::read(SkillSource {
                name: file_name(skill_folder),
                library: library.clone(),
                library_priority,
                skill_md: String::from_utf8_lossy(&bytes).into_owned(),
                ..SkillSource::default()
            }))
        })
        .collect();
    Ok(entries)
}

/// One `M14` finding for each intent of the skill at `position` that does
/// not find it among the first [`INTENT_TOP`] skills.
fn intent_findings(
    table: &KeywordTable,
    index: &Index,
    position: usize,
) -> Result<Vec<Finding>, SkillsError> {
    let Some(metadata) = &index.skills[position].metadata else {
        return Ok(Vec::new());
    };
    let searched = index.skills.len();
    let mut findings = Vec::new();
    for intent in &metadata.intents {
        let ranked = table.rank(intent)?;
        let place = ranked.iter().position(|found| found.skill == position);
        if place.is_some_and(|place| place < INTENT_TOP) {
            continue;
        }
        // The skill's own intents hold every word of the phrase, so it is
        // missing from the list only when the phrase has no word to search.
        let outcome = match place {
            Some(place) => format!("it ranks {}", place + 1),
            None => "no skill matches it".to_owned(),
        };
        findings.push(Finding {
            file: PathBuf::from(SKILL_FILE),
            rule: Rule::M14,
            message: format!(
                "intent \"{intent}\" does not find this skill in the top {INTENT_TOP} of \
                 {searched} skills; {outcome}"
            ),
        });
    }
    Ok(findings)
}

/// The findings of the intent check for each skill, in the order of
/// `skills`. With no pool there is nothing to search among, so none. With a
/// pool of fewer than [`MIN_INTENT_POOL`] skills, it warns on stderr.
///
/// # Errors
/// - [`SkillsError::Missing`] or [`SkillsError::Read`] for a pool folder
///   that is not there or cannot be listed.
/// - [`SkillsError::Search`] if the keyword search fails.
pub(super) fn findings(
    cwd: &Path,
    skills: &[&Skill],
    pools: &[PathBuf],
) -> Result<Vec<Vec<Finding>>, SkillsError> {
    if pools.is_empty() {
        return Ok(vec![Vec::new(); skills.len()]);
    }
    let index = intent_index(cwd, skills, pools)?;
    if index.skills.len() < MIN_INTENT_POOL {
        print_line_to_stderr(&format!(
            "warning: the intent check searched {} skills; a pool of {MIN_INTENT_POOL} or \
             more is needed for a real check",
            index.skills.len()
        ));
    }
    let table = KeywordTable::build(&index)?;
    (0..skills.len())
        .map(|position| intent_findings(&table, &index, position))
        .collect()
}

/// A warning must not stop the check or hide its result, so a failed write
/// to stderr is ignored.
fn print_line_to_stderr(line: &str) {
    let _ = writeln!(io::stderr().lock(), "{line}");
}
