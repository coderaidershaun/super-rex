//! Token counts as short text, shared by `rex watch`, `rex status` and
//! `rex job show`, so every view writes the same figure the same way.

use crate::capture::{Cost, TokenCounts};

/// `950`, `61.0k`, `1.2M`. Counts are cut, not rounded, so `999999` never
/// shows as `1000.0k`.
pub fn short_count(count: u64) -> String {
    if count < 1_000 {
        count.to_string()
    } else if count < 1_000_000 {
        let tenths = count / 100;
        format!("{}.{}k", tenths / 10, tenths % 10)
    } else {
        let tenths = count / 100_000;
        format!("{}.{}M", tenths / 10, tenths % 10)
    }
}

/// `90%`, or `–` when there was no prompt token.
pub fn hit_text(counts: &TokenCounts) -> String {
    counts
        .cache_hit_percent()
        .map_or_else(|| "–".to_owned(), |percent| format!("{percent}%"))
}

/// `$0.004`, `$0.420`, `$12.34`, `$123`, `<$0.001`, or `$0` for nothing spent.
/// Cut, not rounded. `≥` in front when requests on a model with no price are left out.
pub fn cost_text(cost: &Cost) -> String {
    let mills = cost.nanodollars / 1_000_000;
    let amount = match mills {
        0 if cost.nanodollars == 0 => "$0".to_owned(),
        0 => "<$0.001".to_owned(),
        1..1_000 => format!("$0.{mills:03}"),
        1_000..100_000 => format!("${}.{:02}", mills / 1_000, mills % 1_000 / 10),
        _ => format!("${}", mills / 1_000),
    };
    let floor = if cost.unpriced_requests > 0 {
        "≥"
    } else {
        ""
    };
    format!("{floor}{amount}")
}

/// `0.113724`: dollars with six decimals, cut, for agents and scripts.
pub fn cost_usd(cost: &Cost) -> String {
    let micro = cost.nanodollars / 1_000;
    format!("{}.{:06}", micro / 1_000_000, micro % 1_000_000)
}

/// `in 30  cache w 78.4k  cache r 762.6k  out 13.5k  hit 90%`, the one-line
/// form of `rex status` and the job panel of `rex watch`.
pub fn split_text(counts: &TokenCounts) -> String {
    format!(
        "in {}  cache w {}  cache r {}  out {}  hit {}",
        short_count(counts.input),
        short_count(counts.cache_creation),
        short_count(counts.cache_read),
        short_count(counts.output),
        hit_text(counts),
    )
}
