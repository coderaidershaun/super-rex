//! The order of every kept candidate of one request, which each page cuts: tier A, then B, then
//! C, and inside each tier rounds that cover every need once before any need repeats.

use std::collections::BTreeMap;

use super::super::index::Index;
use super::super::request::{FormTerms, Need};
use super::super::response::{Candidate, Tier};

/// Every tier A item, then every tier B item, then every tier C item. Inside
/// a tier, each round gives each need that the earlier items cover least
/// often its best open item, so every need is covered once before any need
/// repeats. A placed skill brings the skills it requires right after it.
pub(crate) fn order(
    index: &Index,
    terms: &FormTerms,
    candidates: Vec<Candidate>,
) -> Vec<Candidate> {
    let keys = need_keys(terms, &candidates);
    let mut placing = Placing {
        index,
        candidates: &candidates,
        placed: vec![false; candidates.len()],
        ordered: Vec::new(),
        times_covered: BTreeMap::new(),
    };
    for tier in [Tier::A, Tier::B, Tier::C] {
        let tier_keys: Vec<&Need> = keys
            .iter()
            .filter(|key| is_keyword_need(key) == (tier == Tier::C))
            .collect();
        placing.place_tier(tier, &tier_keys);
    }
    debug_assert!(placing.placed.iter().all(|is_placed| *is_placed));
    let ordered = placing.ordered;
    ordered
        .into_iter()
        .map(|at| candidates[at].clone())
        .collect()
}

/// The order as it grows: which candidates are placed, in what order, and
/// how many placed items cover each need.
struct Placing<'a> {
    index: &'a Index,
    candidates: &'a [Candidate],
    placed: Vec<bool>,
    ordered: Vec<usize>,
    times_covered: BTreeMap<Need, usize>,
}

impl Placing<'_> {
    /// Rounds over `keys` for the candidates of `tier`, until none is left.
    /// Round `n` gives a need an item only while fewer than `n` placed items
    /// cover it, so a need covered by an earlier tier waits for the others.
    fn place_tier(&mut self, tier: Tier, keys: &[&Need]) {
        let mut round = 1;
        loop {
            let before = self.ordered.len();
            for key in keys {
                if self.times(key) >= round {
                    continue;
                }
                if let Some(pick) = best_under(self.index, self.candidates, &self.placed, key, tier)
                {
                    self.place(pick);
                }
            }
            let is_done = (0..self.candidates.len())
                .all(|at| self.placed[at] || self.candidates[at].tier != tier);
            let most = self.times_covered.values().copied().max().unwrap_or(0);
            if is_done || (self.ordered.len() == before && round > most) {
                return;
            }
            round += 1;
        }
    }

    fn times(&self, key: &Need) -> usize {
        self.times_covered.get(key).copied().unwrap_or(0)
    }

    fn place(&mut self, at: usize) {
        let candidates = self.candidates;
        let index = self.index;
        self.placed[at] = true;
        self.ordered.push(at);
        for need in &candidates[at].needs {
            *self.times_covered.entry(need.clone()).or_insert(0) += 1;
        }
        let entry = &index.skills[candidates[at].skill];
        let requires = entry
            .metadata
            .iter()
            .flat_map(|metadata| &metadata.requires);
        for name in requires {
            let required = (0..candidates.len()).find(|&other| {
                let pulled = &candidates[other];
                !self.placed[other]
                    && index.skills[pulled.skill].name == *name
                    && pulled.is_only_required()
                    && first_kept_requirer(index, candidates, pulled) == Some(at)
            });
            if let Some(required) = required {
                self.place(required);
            }
        }
    }
}

fn is_keyword_need(need: &Need) -> bool {
    matches!(need, Need::Text(_) | Need::Word(_))
}

/// The kept skill that a skill there only for its requirers follows: the
/// first of its requirers still kept, those of tier A or B before tier C, so a
/// keyword result never moves it among the declared skills.
pub(crate) fn first_kept_requirer(
    index: &Index,
    kept: &[Candidate],
    candidate: &Candidate,
) -> Option<usize> {
    let requirers = candidate.needs.iter().filter_map(|need| match need {
        Need::Required(name) => kept
            .iter()
            .position(|other| index.skills[other.skill].name == *name),
        _ => None,
    });
    requirers.min_by_key(|&at| kept[at].tier == Tier::C)
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
    tier: Tier,
) -> Option<usize> {
    let open = (0..candidates.len()).filter(|&at| {
        let candidate = &candidates[at];
        !placed[at] && candidate.tier == tier && candidate.needs.contains(key)
    });
    let entry = |at: usize| &index.skills[candidates[at].skill];
    if tier == Tier::C {
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
