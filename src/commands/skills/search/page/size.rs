//! The size limit that keeps an answer small: the token estimate of a page, and the steps that
//! shorten a page over its budget before items move to the next page.

use super::super::request::SearchRequest;
use super::super::response::RequestResult;
use super::cursor::CursorKey;

/// The estimate counts this many tokens for each five bytes of JSON that is
/// not a hash: 2.5 bytes per token, as measured on answers of words and names.
pub const TEXT_TOKENS_PER_FIVE_BYTES: usize = 2;

/// The estimate counts this many tokens for each eight bytes of a hash of 64
/// hex digits: 1.6 bytes per token, as measured.
pub const HASH_TOKENS_PER_EIGHT_BYTES: usize = 5;

const HASH_DIGITS: usize = 64;

/// The tokens that the compact JSON `text` costs a caller, rounded up.
#[must_use]
pub fn estimated_tokens(text: &str) -> usize {
    let mut hash_bytes = 0;
    let mut run = 0;
    for byte in text.bytes().chain(std::iter::once(b' ')) {
        if byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte) {
            run += 1;
            continue;
        }
        if run == HASH_DIGITS {
            hash_bytes += run;
        }
        run = 0;
    }
    let text_bytes = text.len() - hash_bytes;
    (text_bytes * TEXT_TOKENS_PER_FIVE_BYTES).div_ceil(5)
        + (hash_bytes * HASH_TOKENS_PER_EIGHT_BYTES).div_ceil(8)
}

/// Shrink a page whose estimate is over the request's budget, one step at a
/// time, until it fits: cut each `why` and gap reason to its head, then leave
/// out hashes (unless the request keeps them), word counts and `placement`.
/// `files` and `open_when` always stay. Then move the last items to the next
/// page, down to one item, and mark a page still over as `over_budget`.
pub(crate) fn fit(
    result: &mut RequestResult,
    key: &CursorKey,
    offset: usize,
    request: &SearchRequest,
) {
    let budget = request.token_budget();
    let mut steps: Vec<fn(&mut RequestResult)> = vec![cut_reasons];
    if !request.keep_hashes {
        steps.push(leave_out_hashes);
    }
    steps.push(leave_out_words);
    steps.push(leave_out_placement);
    set_cursor(result, key, offset);
    for step in steps {
        if fits(result, budget) {
            return;
        }
        step(result);
    }
    while !fits(result, budget) && result.items.len() > 1 {
        result.items.pop();
        result.more += 1;
        set_cursor(result, key, offset);
    }
    result.over_budget = !fits(result, budget);
}

fn set_cursor(result: &mut RequestResult, key: &CursorKey, offset: usize) {
    let next = offset + result.items.len();
    result.cursor = (result.more > 0).then(|| key.cursor(next));
}

fn fits(result: &RequestResult, budget: usize) -> bool {
    let text = serde_json::to_string(result).expect("a result of strings and numbers is JSON");
    estimated_tokens(&text) <= budget
}

/// Cut each `why` and each gap reason before its first `; `, so that
/// `declared: artefact unit-test; scope met` keeps `declared: artefact unit-test`.
fn cut_reasons(result: &mut RequestResult) {
    let cut = |text: &mut String| {
        if let Some(head) = text.find("; ") {
            text.truncate(head);
        }
    };
    result.items.iter_mut().for_each(|item| cut(&mut item.why));
    result.gaps.iter_mut().for_each(|gap| cut(&mut gap.reason));
}

fn leave_out_hashes(result: &mut RequestResult) {
    for item in &mut result.items {
        item.sha256 = None;
        for file in &mut item.files {
            file.sha256 = None;
        }
    }
}

fn leave_out_words(result: &mut RequestResult) {
    result.items.iter_mut().for_each(|item| item.words = None);
}

fn leave_out_placement(result: &mut RequestResult) {
    result
        .items
        .iter_mut()
        .for_each(|item| item.placement = None);
}
