//! Lookups A and B: the facts and terms a request form asks for, matched only against what each
//! skill's author claimed, with the scope check of B and no corpus statistics.

use serde_json::{Map, Value};

use super::index::{Claims, Index, Kind, Scope, SkillEntry};
use super::request::{AskedTerm, Facts, FormTerms, Need, RequestForm, TermField, TextQuery};
use super::response::{Candidate, Found, Gap, Question, Tier};
use super::vocabulary::Vocabulary;

/// The language the rule skills are written for, so a task in it needs no
/// gap skill.
pub const BASE_LANGUAGE: &str = "rust";

impl FormTerms {
    /// The form read against the embedded vocabulary. A word the vocabulary
    /// does not hold is listed under `unrecognised`, never refused.
    #[must_use]
    pub fn read(form: &RequestForm) -> FormTerms {
        let vocabulary = Vocabulary::embedded();
        let mut unrecognised = Vec::new();
        let facts = read_facts(vocabulary, &form.facts, &mut unrecognised);
        let mut terms: Vec<AskedTerm> = Vec::new();
        let needs = form.needs.iter().enumerate();
        let needs = needs.map(|(position, need)| text_query(Need::Text(position), need));
        let mut keyword_queries: Vec<TextQuery> = needs.collect();
        for (field, words) in term_fields(form) {
            for sent in words {
                let asked = asked_terms(vocabulary, field, sent);
                if asked.is_empty() {
                    let need = format!("{}:{sent}", field.request_name());
                    unrecognised.push(need.clone());
                    keyword_queries.push(text_query(Need::Word(need), sent));
                }
                for asked in asked {
                    let key = (asked.field, &asked.term);
                    let is_new = terms.iter().all(|known| (known.field, &known.term) != key);
                    if is_new {
                        terms.push(asked);
                    }
                }
            }
        }
        let quotes = form.evidence.iter().map(|evidence| &evidence.quote);
        let task_words = form.needs.iter().chain(quotes).flat_map(|text| words(text));
        FormTerms {
            id: form.id.clone(),
            facts,
            terms,
            needs: form.needs.clone(),
            task_words: task_words.collect(),
            unrecognised,
            keyword_queries,
        }
    }
}

/// Lookup A: the skills whose `claims.facts` hold for the form.
#[must_use]
pub fn lookup_a(index: &Index, form: &FormTerms) -> Found {
    let mut found = Found::default();
    for (position, entry) in index.skills.iter().enumerate() {
        found
            .candidates
            .extend(rule_candidate(position, entry, form));
    }
    found
}

/// Lookup B: the skills that claim a term the form asks for, after the
/// scope check. A skill that only its scope leaves out is a gap.
#[must_use]
pub fn lookup_b(index: &Index, form: &FormTerms) -> Found {
    let mut found = Found::default();
    for (position, entry) in index.skills.iter().enumerate() {
        let Some(metadata) = entry.metadata.as_ref() else {
            continue;
        };
        let claimed =
            |asked: &&AskedTerm| claimed_terms(&metadata.claims, asked.field).contains(&asked.term);
        let matched: Vec<&AskedTerm> = form.terms.iter().filter(claimed).collect();
        if matched.is_empty() {
            continue;
        }
        let checks = scope_checks(&metadata.scope, &form.facts);
        if checks.iter().all(|check| check.answer() != Answer::No) {
            let candidate = declared_candidate(position, &matched, &checks);
            found.candidates.push(candidate);
        } else if index.is_served(position) {
            let gaps = scope_gaps(entry, &matched, &checks);
            found.gaps.extend(gaps);
        }
    }
    found
}

/// The skills that a found skill requires and `found` does not hold, each with one
/// `Need::Required` per requirer in candidate order, and a gap for each name no served skill has.
#[must_use]
pub fn required_skills(index: &Index, found: &[Candidate]) -> Found {
    let mut requirers: Vec<Candidate> = found.to_vec();
    let mut added = Found::default();
    let mut next = 0;
    while let Some(requirer) = requirers.get(next).cloned() {
        next += 1;
        let entry = &index.skills[requirer.skill];
        let metadata = entry.metadata.iter();
        for name in metadata.flat_map(|metadata| &metadata.requires) {
            let Some(position) = served_position(index, name) else {
                let reason = format!("required by {}, and no served library holds it", entry.name);
                added.gaps.push(Gap {
                    skill: Some(entry.name.clone()),
                    ..gap(format!("requires:{name}"), reason)
                });
                continue;
            };
            let need = Need::Required(entry.name.clone());
            let mut added_before = added.candidates.iter_mut();
            if let Some(known) = added_before.find(|known| known.skill == position) {
                if !known.needs.contains(&need) {
                    known.needs.push(need);
                }
                continue;
            }
            if found.iter().any(|known| known.skill == position) {
                continue;
            }
            let needs = vec![need];
            let why = format!("required by {}", entry.name);
            let candidate = Candidate {
                questions: requirer.questions.clone(),
                ..candidate(position, requirer.tier, needs, why)
            };
            requirers.push(candidate.clone());
            added.candidates.push(candidate);
        }
    }
    added
}

/// A gap for each language of the form, other than [`BASE_LANGUAGE`], that
/// no served skill claims under `claims.facts.languages`.
#[must_use]
pub fn language_gaps(index: &Index, form: &FormTerms) -> Vec<Gap> {
    if form.facts.writes_code != Some(true) {
        return Vec::new();
    }
    let languages = form.facts.languages.iter();
    languages
        .filter(|language| language.as_str() != BASE_LANGUAGE)
        .filter(|language| !language_is_claimed(index, language))
        .map(|language| {
            let reason = format!("no skill claims the language {language}");
            gap(format!("languages:{language}"), reason)
        })
        .collect()
}

fn language_is_claimed(index: &Index, language: &String) -> bool {
    (0..index.skills.len()).any(|position| {
        let metadata = index.skills[position].metadata.as_ref();
        let claims = metadata.is_some_and(|it| it.claims.facts.languages.contains(language));
        claims && index.is_served(position)
    })
}

fn served_position(index: &Index, name: &str) -> Option<usize> {
    (0..index.skills.len())
        .find(|&position| index.skills[position].name == name && index.is_served(position))
}

fn gap(need: String, reason: String) -> Gap {
    Gap {
        need,
        reason,
        skill: None,
        library: None,
        not_when: None,
    }
}

fn candidate(skill: usize, tier: Tier, needs: Vec<Need>, why: String) -> Candidate {
    Candidate {
        skill,
        tier,
        needs,
        why,
        questions: Vec::new(),
        through_alias: false,
        relevance: None,
    }
}

fn text_query(need: Need, text: &str) -> TextQuery {
    TextQuery {
        need,
        text: text.to_owned(),
    }
}

fn term_fields(form: &RequestForm) -> [(TermField, &[String]); 4] {
    [
        (TermField::Quality, form.must_have.as_slice()),
        (TermField::Artefact, form.artefacts.as_slice()),
        (TermField::Activity, form.activity.as_slice()),
        (TermField::Touches, form.touches.as_slice()),
    ]
}

/// The term the word names in the group of `field`, or one term for each
/// alias row of that group whose word it is. Empty for a word of no term.
fn asked_terms(vocabulary: &Vocabulary, field: TermField, sent: &str) -> Vec<AskedTerm> {
    let word = normalised(sent);
    if vocabulary.has_term(field.group(), &word) {
        let from_alias = None;
        return vec![AskedTerm {
            field,
            term: word,
            from_alias,
        }];
    }
    let aliases = vocabulary.aliases_of(field.group(), &word).into_iter();
    aliases
        .map(|alias| AskedTerm {
            field,
            term: alias.term.clone(),
            from_alias: Some(sent.to_owned()),
        })
        .collect()
}

/// The word in lower case with single spaces between its parts, as the
/// vocabulary writes its terms and alias words.
fn normalised(word: &str) -> String {
    let parts: Vec<&str> = word.split_whitespace().collect();
    parts.join(" ").to_lowercase()
}

/// The lower-case runs of letters and digits of a text.
fn words(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let runs = lower.split(|character: char| !character.is_alphanumeric());
    let runs = runs.filter(|word| !word.is_empty());
    runs.map(str::to_owned).collect()
}

fn read_facts(
    vocabulary: &Vocabulary,
    sent: &Map<String, Value>,
    unrecognised: &mut Vec<String>,
) -> Facts {
    let mut facts = Facts::default();
    for (name, value) in sent {
        let is_read = vocabulary.has_fact(name)
            && read_fact(&mut facts, name, value, vocabulary, unrecognised).is_some();
        if !is_read {
            unrecognised.push(format!("facts:{name}"));
        }
    }
    facts
}

/// Puts one sent fact into `facts`. `None` when the value has the wrong type
/// for the fact, or the fact is one that `Facts` cannot hold.
/// A closed fact keeps only the values the vocabulary allows, and lists each
/// other value as `<fact>:<value>`; an open fact keeps its value as sent.
fn read_fact(
    facts: &mut Facts,
    name: &str,
    value: &Value,
    vocabulary: &Vocabulary,
    unrecognised: &mut Vec<String>,
) -> Option<()> {
    let mut allowed = |sent: &str| {
        let Some(allowed) = vocabulary.allowed_values(name) else {
            return Some(sent.to_owned());
        };
        let value = normalised(sent);
        let is_allowed = allowed.contains(&value.as_str());
        if !is_allowed {
            unrecognised.push(format!("{name}:{sent}"));
        }
        is_allowed.then_some(value)
    };
    match name {
        "writes_code" => facts.writes_code = Some(value.as_bool()?),
        "role" => facts.role = Some(value.as_str()?.to_owned()),
        "latency_scale" => facts.latency_scale = allowed(value.as_str()?),
        "languages" => {
            let languages = text_list(value)?.into_iter();
            facts.languages = languages
                .filter_map(|language| allowed(&language))
                .collect();
        }
        "repo_has" => facts.repo_has = text_list(value)?,
        "repo_dependencies" => facts.repo_dependencies = text_list(value)?,
        _ => return None,
    }
    Some(())
}

fn text_list(value: &Value) -> Option<Vec<String>> {
    let items = value.as_array()?.iter();
    items.map(|item| item.as_str().map(str::to_owned)).collect()
}

/// One fact of a claim, and `<fact>=<shared values>` when it holds for the
/// form, or `None` when it does not hold.
struct ClaimedFact {
    name: &'static str,
    held: Option<String>,
}

impl ClaimedFact {
    fn is_evidence(&self) -> bool {
        matches!(self.name, "repo_has" | "repo_dependencies")
    }
}

/// Each fact as a list of value texts, in the order writes_code, languages,
/// role, repo_has, repo_dependencies. A fact left out is an empty list.
fn fact_values(facts: &Facts) -> [(&'static str, Vec<String>); 5] {
    [
        ("writes_code", texts(facts.writes_code)),
        ("languages", facts.languages.clone()),
        ("role", texts(&facts.role)),
        ("repo_has", facts.repo_has.clone()),
        ("repo_dependencies", facts.repo_dependencies.clone()),
    ]
}

/// The facts a claim names. One holds when the form's values share at least
/// one with the claim's, so a fact the form leaves out does not hold.
fn claimed_facts(claimed: &Facts, form: &Facts) -> Vec<ClaimedFact> {
    let pairs = fact_values(claimed).into_iter().zip(fact_values(form));
    pairs
        .filter(|((_, claimed), _)| !claimed.is_empty())
        .map(|((name, claimed), (_, sent))| {
            let shared = claimed.into_iter().filter(|value| sent.contains(value));
            let shared: Vec<String> = shared.collect();
            let held = (!shared.is_empty()).then(|| format!("{name}={}", shared.join(",")));
            ClaimedFact { name, held }
        })
        .collect()
}

fn rule_candidate(position: usize, entry: &SkillEntry, form: &FormTerms) -> Option<Candidate> {
    let metadata = entry.metadata.as_ref()?;
    let facts = claimed_facts(&metadata.claims.facts, &form.facts);
    let is_tool = metadata.kind == Kind::Tool;
    let mut required = facts.iter().filter(|fact| !(is_tool && fact.is_evidence()));
    if facts.is_empty() || !required.all(|fact| fact.held.is_some()) {
        return None;
    }
    let mut evidence = facts.iter().filter(|fact| fact.is_evidence());
    let named_in_task = match is_tool && !evidence.any(|fact| fact.held.is_some()) {
        true => Some(tool_named_in_task(entry, form)?),
        false => None,
    };
    let held: Vec<&ClaimedFact> = facts.iter().filter(|fact| fact.held.is_some()).collect();
    let names: Vec<&str> = held.iter().map(|fact| fact.name).collect();
    let shown = held.iter().filter_map(|fact| fact.held.as_deref());
    let shown: Vec<&str> = shown.collect();
    let need = Some(names.join("+")).filter(|need| !need.is_empty());
    let need = need.unwrap_or_else(|| "named".to_owned());
    let mut reasons = Vec::from_iter((!shown.is_empty()).then(|| shown.join(", ")));
    reasons.extend(named_in_task.map(|name| format!("tool named in the task: {name}")));
    let why = format!("rule: {}", reasons.join("; "));
    Some(candidate(position, Tier::A, vec![Need::Fact(need)], why))
}

/// The first name of a tool, its skill name and then each dependency it
/// claims, whose words occur as a run in the task's words.
fn tool_named_in_task<'a>(entry: &'a SkillEntry, form: &FormTerms) -> Option<&'a str> {
    let metadata = entry.metadata.iter();
    let dependencies = metadata.flat_map(|metadata| &metadata.claims.facts.repo_dependencies);
    let mut names = std::iter::once(&entry.name).chain(dependencies);
    let name = names.find(|name| {
        let name_words = words(name);
        let mut runs = form.task_words.windows(name_words.len().max(1));
        !name_words.is_empty() && runs.any(|run| run == name_words.as_slice())
    });
    name.map(String::as_str)
}

fn claimed_terms(claims: &Claims, field: TermField) -> &[String] {
    match field {
        TermField::Quality => &claims.quality,
        TermField::Artefact => &claims.artefact,
        TermField::Activity => &claims.activity,
        TermField::Touches => &claims.touches,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Answer {
    Yes,
    Unknown,
    No,
}

/// The scope check of one key: what the scope allows and what the form
/// sent for it.
struct ScopeCheck {
    key: &'static str,
    allowed: Vec<String>,
    sent: Vec<String>,
}

impl ScopeCheck {
    fn answer(&self) -> Answer {
        if self.sent.is_empty() {
            Answer::Unknown
        } else if self.sent.iter().any(|value| self.allowed.contains(value)) {
            Answer::Yes
        } else {
            Answer::No
        }
    }

    fn refusal(&self) -> String {
        let sent = self.sent.join(",");
        format!("{} {sent} not in {}", self.key, self.allowed.join(","))
    }
}

/// One check for each scope key with a non-empty list, in the order
/// writes_code, languages, latency_scale.
fn scope_checks(scope: &Scope, form: &Facts) -> Vec<ScopeCheck> {
    let (code, sent_code) = (texts(&scope.writes_code), texts(form.writes_code));
    let (scale, sent_scale) = (scope.latency_scale.clone(), texts(&form.latency_scale));
    let mut keys = vec![
        ("writes_code", code, sent_code),
        ("languages", scope.languages.clone(), form.languages.clone()),
        ("latency_scale", scale, sent_scale),
    ];
    keys.retain(|(_, allowed, _)| !allowed.is_empty());
    keys.into_iter()
        .map(|(key, allowed, sent)| ScopeCheck { key, allowed, sent })
        .collect()
}

fn texts<T: ToString>(values: impl IntoIterator<Item = T>) -> Vec<String> {
    values.into_iter().map(|value| value.to_string()).collect()
}

fn scope_gaps(entry: &SkillEntry, matched: &[&AskedTerm], checks: &[ScopeCheck]) -> Vec<Gap> {
    let not_when = entry
        .metadata
        .as_ref()
        .and_then(|metadata| metadata.not_when.clone());
    let refused = checks.iter().filter(|check| check.answer() == Answer::No);
    let refusals: Vec<String> = refused.map(ScopeCheck::refusal).collect();
    let reason = format!("scope: {}", refusals.join("; "));
    let gaps = matched.iter().map(|asked| {
        let need = format!("{}:{}", asked.field.request_name(), asked.term);
        Gap {
            skill: Some(entry.name.clone()),
            library: Some(entry.library.clone()),
            not_when: not_when.clone(),
            ..gap(need, reason.clone())
        }
    });
    gaps.collect()
}

fn declared_candidate(position: usize, matched: &[&AskedTerm], checks: &[ScopeCheck]) -> Candidate {
    let declared: Vec<String> = matched
        .iter()
        .map(|asked| match &asked.from_alias {
            Some(word) => format!("{} {} (from {word})", asked.field.claim_name(), asked.term),
            None => format!("{} {}", asked.field.claim_name(), asked.term),
        })
        .collect();
    let unknown = checks
        .iter()
        .filter(|check| check.answer() == Answer::Unknown);
    let unknown: Vec<&ScopeCheck> = unknown.collect();
    let mut why = format!("declared: {}", declared.join(", "));
    if !unknown.is_empty() {
        let keys: Vec<&str> = unknown.iter().map(|check| check.key).collect();
        why.push_str(&format!("; scope unknown: {}", keys.join(", ")));
    } else if !checks.is_empty() {
        why.push_str("; scope met");
    }
    let questions = unknown.iter().map(|check| Question::Scope {
        fact: check.key.to_owned(),
        values: check.allowed.clone(),
    });
    let needs = matched.iter().map(|asked| Need::Term {
        field: asked.field,
        term: asked.term.clone(),
    });
    Candidate {
        questions: questions.collect(),
        through_alias: matched.iter().all(|asked| asked.from_alias.is_some()),
        ..candidate(position, Tier::B, needs.collect(), why)
    }
}
