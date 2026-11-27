//! The answer of one search call: lookups A, B and C for each request form, then merge, drops,
//! filters, gaps, and the order that covers every need once before any slot repeats.

use std::collections::BTreeSet;

use super::claims::{language_gaps, lookup_a, lookup_b, required_skills};
use super::index::{Index, IndexVersion, SkillEntry};
use super::keyword::lookup_c;
use super::page::{CursorKey, fit, item, write_confirms};
use super::request::{FormTerms, MAX_NEEDS, Need, RequestForm, SearchRequest, TextQuery};
use super::response::{
    Candidate, Dropped, Gap, RequestResult, SearchError, SearchResponse, Slot, Tier,
};
use super::vocabulary::{TermGroup, Vocabulary};

/// The one function that `rex skills search` and the `search` tool of
/// `rex skills serve` both call.
///
/// # Errors
/// [`SearchError::BadArguments`] for a request that breaks a rule of the
/// form, [`SearchError::BadCursor`] or [`SearchError::StaleCursor`] for a
/// cursor that cannot continue, and [`SearchError::KeywordSearch`] when
/// SQLite fails.
pub fn search(
    index: &Index,
    version: &IndexVersion,
    request: &SearchRequest,
) -> Result<SearchResponse, SearchError> {
    check_arguments(request)?;
    let results = request.requests.iter().map(|form| {
        let key = CursorKey::new(version, request, form);
        answer_form(index, request, form, &key)
    });
    Ok(SearchResponse {
        index_version: version.clone(),
        results: results.collect::<Result<_, _>>()?,
    })
}

fn check_arguments(request: &SearchRequest) -> Result<(), SearchError> {
    let bad = |message: String| Err(SearchError::BadArguments { message });
    if request.requests.is_empty() {
        return bad("`requests` is empty; give at least one request form".to_owned());
    }
    let mut ids = BTreeSet::new();
    for (position, form) in request.requests.iter().enumerate() {
        if form.id.is_empty() {
            return bad(format!("`id` of request {} is empty", position + 1));
        }
        if !ids.insert(form.id.as_str()) {
            return bad(format!(
                "`id` {:?} is given to more than one request",
                form.id
            ));
        }
        if form.needs.len() > MAX_NEEDS {
            let count = form.needs.len();
            return bad(format!(
                "`needs` of request {:?} holds {count} items; at most {MAX_NEEDS} are allowed",
                form.id
            ));
        }
    }
    if request.limit == Some(0) {
        return bad("`limit` is 0; give 1 or more".to_owned());
    }
    if request.cursor.is_some() && request.requests.len() != 1 {
        let count = request.requests.len();
        return bad(format!(
            "`cursor` continues one request, but `requests` holds {count}"
        ));
    }
    Ok(())
}

/// What one form found, before the order and the page.
struct Found {
    kept: Vec<Candidate>,
    gaps: Vec<Gap>,
    unrecognised: Vec<String>,
    dropped: Vec<Dropped>,
}

fn answer_form(
    index: &Index,
    request: &SearchRequest,
    form: &RequestForm,
    key: &CursorKey,
) -> Result<RequestResult, SearchError> {
    let offset = match &request.cursor {
        Some(cursor) => key.read(cursor)?,
        None => 0,
    };
    let terms = FormTerms::read(form);
    let found = find(index, request, &terms)?;
    let ordered = order(index, &terms, found.kept);
    if offset > ordered.len() {
        return Err(SearchError::BadCursor {
            reason: "its offset is past the end of the list".to_owned(),
        });
    }
    let end = ordered.len().min(offset.saturating_add(request.limit()));
    let mut page = Vec::new();
    for candidate in &ordered[offset..end] {
        page.push(item(index, &terms, candidate)?);
    }
    write_confirms(&mut page, &ordered[offset..end]);
    let mut result = RequestResult {
        request: form.id.clone(),
        items: page,
        more: ordered.len() - end,
        cursor: None,
        gaps: found.gaps,
        unrecognised: found.unrecognised,
        dropped: found.dropped,
    };
    fit(&mut result, key, offset);
    Ok(result)
}

/// Steps 2 to 7 of the answer: lookups, merge, drops, required skills, gaps
/// and filters.
fn find(index: &Index, request: &SearchRequest, terms: &FormTerms) -> Result<Found, SearchError> {
    let declared = lookup_b(index, terms);
    let mut merged = Vec::new();
    let all = lookup_a(index, terms).candidates.into_iter();
    let all = all
        .chain(declared.candidates)
        .chain(lookup_c(index, terms)?.candidates);
    for candidate in all {
        absorb(&mut merged, candidate);
    }
    let (mut kept, mut gone) = split_served(index, merged);
    let required = required_skills(index, &kept);
    let (added, gone_too) = split_served(index, required.candidates);
    kept.extend(added);
    gone.extend(gone_too);
    gone.sort_by_key(|candidate| candidate.skill);

    let mut gaps = term_gaps(terms, declared.gaps, &kept, &gone);
    gaps.extend(need_gaps(terms, &kept));
    gaps.extend(language_gaps(index, terms));
    gaps.extend(required.gaps);

    let mut unrecognised = terms.unrecognised.clone();
    let kinds = request.kinds.as_ref();
    let categories = request.categories.as_ref();
    list_unknown_filter_words(TermGroup::Kinds, "kinds", kinds, &mut unrecognised);
    list_unknown_filter_words(
        TermGroup::Categories,
        "categories",
        categories,
        &mut unrecognised,
    );
    kept.retain(|candidate| {
        is_only_required(candidate) || passes(&index.skills[candidate.skill], kinds, categories)
    });
    leave_out_lone_required(index, &mut kept);
    follow_first_kept_requirer(index, &mut kept);

    let dropped = gone.iter().map(|candidate| dropped(index, candidate));
    Ok(Found {
        kept,
        gaps,
        unrecognised,
        dropped: dropped.collect(),
    })
}

/// Merge `candidate` into the candidate of the same skill, if any. The
/// lookups come in order A, B, C, so the held candidate has the best tier and
/// keeps its `why`. An A item takes B's scope questions; a C question is kept
/// only when no better tier found the skill.
fn absorb(merged: &mut Vec<Candidate>, candidate: Candidate) {
    let Some(held) = merged.iter_mut().find(|held| held.skill == candidate.skill) else {
        merged.push(candidate);
        return;
    };
    for need in candidate.needs {
        if !held.needs.contains(&need) {
            held.needs.push(need);
        }
    }
    held.relevance = match (held.relevance, candidate.relevance) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (left, right) => left.or(right),
    };
    match (held.tier, candidate.tier) {
        (Tier::A, Tier::B) => held.questions = candidate.questions,
        (Tier::C, Tier::C) => {
            for question in candidate.questions {
                if !held.questions.contains(&question) {
                    held.questions.push(question);
                }
            }
        }
        _ => {}
    }
}

/// The served candidates, and the shadowed or replaced ones for `dropped`.
fn split_served(index: &Index, candidates: Vec<Candidate>) -> (Vec<Candidate>, Vec<Candidate>) {
    candidates
        .into_iter()
        .partition(|candidate| index.is_served(candidate.skill))
}

fn dropped(index: &Index, candidate: &Candidate) -> Dropped {
    let entry = &index.skills[candidate.skill];
    let reason = match (&entry.shadowed_by, index.replaced_by(candidate.skill)) {
        (Some(library), _) => format!("shadowed by {} of library {library}", entry.name),
        (None, Some(replacer)) => format!("replaced by {}", replacer.name),
        (None, None) => unreachable!("only a shadowed or replaced skill is dropped"),
    };
    Dropped {
        skill: entry.name.clone(),
        library: entry.library.clone(),
        reason,
    }
}

/// For each asked term: the skills only their scope left out, or a gap when
/// no kept skill fills it.
fn term_gaps(
    terms: &FormTerms,
    scope_gaps: Vec<Gap>,
    kept: &[Candidate],
    gone: &[Candidate],
) -> Vec<Gap> {
    let mut gaps = Vec::new();
    let mut seen = Vec::new();
    for asked in &terms.terms {
        let need = Need::Term {
            field: asked.field,
            term: asked.term.clone(),
        };
        if seen.contains(&need) {
            continue;
        }
        seen.push(need.clone());
        let label = format!("{}:{}", asked.field.request_name(), asked.term);
        let scope: Vec<&Gap> = scope_gaps.iter().filter(|gap| gap.need == label).collect();
        let fills = |candidate: &Candidate| candidate.needs.contains(&need);
        if !scope.is_empty() {
            gaps.extend(scope.into_iter().cloned());
        } else if !kept.iter().any(fills) {
            let reason = if gone.iter().any(fills) {
                "only skills under dropped claim it"
            } else {
                "no skill claims it"
            };
            gaps.push(plain_gap(label, reason));
        }
    }
    gaps
}

/// A gap for each need and unrecognised word that no kept skill fills.
fn need_gaps(terms: &FormTerms, kept: &[Candidate]) -> Vec<Gap> {
    let queries = terms.keyword_queries.iter();
    let missed = queries.filter(|query| {
        !kept
            .iter()
            .any(|candidate| candidate.needs.contains(&query.need))
    });
    let label = |query: &TextQuery| match &query.need {
        Need::Word(word) => word.clone(),
        _ => query.text.clone(),
    };
    missed
        .map(|query| plain_gap(label(query), "no keyword match"))
        .collect()
}

fn plain_gap(need: String, reason: &str) -> Gap {
    Gap {
        need,
        reason: reason.to_owned(),
        skill: None,
        library: None,
        not_when: None,
    }
}

/// A word of a `kinds` or `categories` filter that is not a term of its
/// group is listed as unrecognised. It still filters.
fn list_unknown_filter_words(
    group: TermGroup,
    field: &str,
    words: Option<&Vec<String>>,
    unrecognised: &mut Vec<String>,
) {
    for word in words.into_iter().flatten() {
        if !Vocabulary::embedded().has_term(group, word) {
            unrecognised.push(format!("{field}:{word}"));
        }
    }
}

fn passes(
    entry: &SkillEntry,
    kinds: Option<&Vec<String>>,
    categories: Option<&Vec<String>>,
) -> bool {
    let metadata = entry.metadata.as_ref();
    let kind = metadata.map(|metadata| Slot::Kind(metadata.kind).to_string());
    let category = metadata.map(|metadata| metadata.category.clone());
    let holds = |filter: Option<&Vec<String>>, value: Option<String>| match (filter, value) {
        (None, _) => true,
        (Some(filter), Some(value)) => filter.contains(&value),
        (Some(_), None) => false,
    };
    holds(kinds, kind) && holds(categories, category)
}

/// True for a skill that is there only because other skills require it. It
/// passes the filters and takes its place right after a skill that requires it.
fn is_only_required(candidate: &Candidate) -> bool {
    let mut needs = candidate.needs.iter();
    needs.all(|need| matches!(need, Need::Required(_)))
}

/// The kept skill that a skill there only for its requirers follows: the
/// first of its requirers still kept, those of tier A or B before tier C, so a
/// keyword result never moves it among the declared skills.
fn first_kept_requirer(index: &Index, kept: &[Candidate], candidate: &Candidate) -> Option<usize> {
    let requirers = candidate.needs.iter().filter_map(|need| match need {
        Need::Required(name) => kept
            .iter()
            .position(|other| index.skills[other.skill].name == *name),
        _ => None,
    });
    requirers.min_by_key(|&at| kept[at].tier == Tier::C)
}

/// A skill there only for its requirers takes its tier, `why` and questions
/// from the first requirer still kept, not from one a filter left out.
fn follow_first_kept_requirer(index: &Index, kept: &mut [Candidate]) {
    for at in 0..kept.len() {
        if !is_only_required(&kept[at]) {
            continue;
        }
        let Some(requirer) = first_kept_requirer(index, kept, &kept[at]) else {
            continue;
        };
        let requirer = kept[requirer].clone();
        let follower = &mut kept[at];
        follower.tier = requirer.tier;
        follower.why = format!("required by {}", index.skills[requirer.skill].name);
        follower.questions = requirer.questions;
    }
}

/// Leave out each skill that is there only because another skill requires
/// it, once no skill that requires it is left.
fn leave_out_lone_required(index: &Index, kept: &mut Vec<Candidate>) {
    loop {
        let names: Vec<String> = kept
            .iter()
            .map(|candidate| index.skills[candidate.skill].name.clone())
            .collect();
        let before = kept.len();
        kept.retain(|candidate| {
            let mut requirers = candidate.needs.iter().map(|need| match need {
                Need::Required(requirer) => Some(requirer),
                _ => None,
            });
            !requirers.all(|requirer| requirer.is_some_and(|name| !names.contains(name)))
        });
        if kept.len() == before {
            return;
        }
    }
}

/// The order of 6.9: rounds that each cover every need key once, a placed
/// skill bringing the skills it requires right after it.
fn order(index: &Index, terms: &FormTerms, candidates: Vec<Candidate>) -> Vec<Candidate> {
    let keys = need_keys(terms, &candidates);
    let mut placed = vec![false; candidates.len()];
    let mut ordered = Vec::new();
    loop {
        let mut covered: Vec<Need> = Vec::new();
        let before = ordered.len();
        for key in &keys {
            if covered.contains(key) {
                continue;
            }
            if let Some(pick) = best_under(index, &candidates, &placed, key) {
                place(
                    index,
                    &candidates,
                    pick,
                    &mut placed,
                    &mut covered,
                    &mut ordered,
                );
            }
        }
        if ordered.len() == before {
            break;
        }
    }
    debug_assert!(placed.iter().all(|is_placed| *is_placed));
    ordered
        .into_iter()
        .map(|at| candidates[at].clone())
        .collect()
}

fn need_keys(terms: &FormTerms, candidates: &[Candidate]) -> Vec<Need> {
    let facts = candidates.iter().flat_map(|candidate| &candidate.needs);
    let facts = facts.filter(|need| matches!(need, Need::Fact(_))).cloned();
    let asked = terms.terms.iter().map(|asked| Need::Term {
        field: asked.field,
        term: asked.term.clone(),
    });
    let texts = terms.keyword_queries.iter().map(|query| query.need.clone());
    let mut keys = Vec::new();
    for key in facts.chain(asked).chain(texts) {
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    keys
}

fn best_under(
    index: &Index,
    candidates: &[Candidate],
    placed: &[bool],
    key: &Need,
) -> Option<usize> {
    let keyword_key = matches!(key, Need::Text(_) | Need::Word(_));
    let open = (0..candidates.len()).filter(|&at| {
        let candidate = &candidates[at];
        !placed[at] && candidate.needs.contains(key) && (candidate.tier == Tier::C) == keyword_key
    });
    let entry = |at: usize| &index.skills[candidates[at].skill];
    if keyword_key {
        open.min_by(|&left, &right| {
            let relevance = |at: usize| candidates[at].relevance.unwrap_or(f64::MIN);
            relevance(right)
                .total_cmp(&relevance(left))
                .then_with(|| entry(left).name.cmp(&entry(right).name))
                .then_with(|| {
                    entry(left)
                        .library_priority
                        .cmp(&entry(right).library_priority)
                })
        })
    } else {
        open.min_by_key(|&at| {
            let candidate = &candidates[at];
            let held = candidate.needs.iter();
            let held = held.filter(|need| matches!(need, Need::Fact(_) | Need::Term { .. }));
            (
                entry(at).library_priority,
                std::cmp::Reverse(held.count()),
                candidate.through_alias,
                entry(at).name.clone(),
            )
        })
    }
}

fn place(
    index: &Index,
    candidates: &[Candidate],
    at: usize,
    placed: &mut [bool],
    covered: &mut Vec<Need>,
    ordered: &mut Vec<usize>,
) {
    placed[at] = true;
    ordered.push(at);
    covered.extend(candidates[at].needs.iter().cloned());
    let entry = &index.skills[candidates[at].skill];
    let requires = entry
        .metadata
        .iter()
        .flat_map(|metadata| &metadata.requires);
    for name in requires {
        let required = (0..candidates.len()).find(|&other| {
            let pulled = &candidates[other];
            !placed[other]
                && index.skills[pulled.skill].name == *name
                && is_only_required(pulled)
                && first_kept_requirer(index, candidates, pulled) == Some(at)
        });
        if let Some(required) = required {
            place(index, candidates, required, placed, covered, ordered);
        }
    }
}
