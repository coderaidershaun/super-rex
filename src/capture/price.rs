//! What each API request cost in US dollars, at Anthropic's first-party list prices,
//! from the model and the usage one transcript record gives.

use std::ops::AddAssign;

use serde_json::Value;

use super::transcripts::text_at;

/// A cost in billionths of a dollar, so every list price is a whole number per token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Cost {
    pub nanodollars: u64,
    /// Requests that used tokens on a model this table has no price for. Their cost is
    /// not in `nanodollars`, so the figure is then a lower bound.
    pub unpriced_requests: u64,
}

impl Cost {
    pub fn dollars(&self) -> f64 {
        // A cost far beyond any real one loses only digits it never shows.
        self.nanodollars as f64 / 1e9
    }
}

/// The counts come from files that anyone can write, so a sum stops at the largest
/// value instead of overflowing.
impl AddAssign for Cost {
    fn add_assign(&mut self, other: Self) {
        self.nanodollars = self.nanodollars.saturating_add(other.nanodollars);
        self.unpriced_requests = self
            .unpriced_requests
            .saturating_add(other.unpriced_requests);
    }
}

/// The price of one token of each kind, in billionths of a dollar ($ per million
/// tokens × 1000).
#[derive(Debug, Clone, Copy)]
struct Rates {
    input: u64,
    cache_write_5m: u64,
    cache_write_1h: u64,
    cache_read: u64,
    output: u64,
}

const fn rates(input: u64, cache_read: u64, output: u64) -> Rates {
    // Cache writes cost 1.25 times the input price for 5 minutes, 2 times for 1 hour.
    Rates {
        input,
        cache_write_5m: input * 5 / 4,
        cache_write_1h: input * 2,
        cache_read,
        output,
    }
}

/// Above this many prompt tokens in one request, Claude Haiku 5.5 charges its second
/// rate card.
const HAIKU_5_5_SMALL_PROMPT: u64 = 100_000;

/// List prices of the Claude API, as of 2026-11. A model that is not here is unpriced.
fn rates_of(model: &str, prompt_tokens: u64) -> Option<Rates> {
    let rates = match model {
        "claude-fable-5-2" | "claude-mythos-5-2" => rates(8_000, 200, 40_000),
        "claude-fable-5-1" | "claude-mythos-5-1" => rates(10_000, 250, 50_000),
        "claude-fable-5" | "claude-mythos-5" => rates(10_000, 1_000, 50_000),
        "claude-opus-5-5" => rates(4_000, 200, 20_000),
        "claude-opus-5" | "claude-opus-4-8" | "claude-opus-4-7" | "claude-opus-4-6" => {
            rates(5_000, 500, 25_000)
        }
        "claude-sonnet-5-5" => rates(2_000, 100, 10_000),
        "claude-sonnet-5" => rates(2_000, 200, 10_000),
        "claude-sonnet-4-6" => rates(3_000, 300, 15_000),
        "claude-haiku-5-5" if prompt_tokens > HAIKU_5_5_SMALL_PROMPT => rates(500, 50, 2_500),
        "claude-haiku-5-5" => rates(100, 10, 500),
        "claude-haiku-4-5" => rates(1_000, 100, 5_000),
        _ => return None,
    };
    Some(rates)
}

/// Fast mode doubles every price, on the models that offer it.
fn has_fast_mode(model: &str) -> bool {
    matches!(
        model,
        "claude-opus-5-5" | "claude-fable-5-2" | "claude-mythos-5-2"
    )
}

/// The cost of the request that one assistant `message` records. A request that used
/// no token costs nothing, whatever its model.
pub(super) fn cost_of(message: &Value) -> Cost {
    let usage = message.get("usage");
    let count = |path: &[&str]| {
        path.iter()
            .fold(usage, |value, key| value.and_then(|value| value.get(key)))
            .and_then(Value::as_u64)
            .unwrap_or(0)
    };
    let input = count(&["input_tokens"]);
    let output = count(&["output_tokens"]);
    let cache_read = count(&["cache_read_input_tokens"]);
    let cache_write = count(&["cache_creation_input_tokens"]);
    let cache_write_1h = count(&["cache_creation", "ephemeral_1h_input_tokens"]).min(cache_write);
    // A write that the record does not split by duration is priced as the 5-minute default.
    let cache_write_5m = cache_write - cache_write_1h;
    if input == 0 && output == 0 && cache_read == 0 && cache_write == 0 {
        return Cost::default();
    }
    let model = text_at(message, "model").unwrap_or_default();
    let prompt = input.saturating_add(cache_read).saturating_add(cache_write);
    let fast = usage.and_then(|usage| text_at(usage, "speed")) == Some("fast");
    let rates = rates_of(model, prompt).filter(|_| !fast || has_fast_mode(model));
    let Some(rates) = rates else {
        return Cost {
            nanodollars: 0,
            unpriced_requests: 1,
        };
    };
    let factor = if fast { 2 } else { 1 };
    let nanodollars = [
        (input, rates.input),
        (cache_write_5m, rates.cache_write_5m),
        (cache_write_1h, rates.cache_write_1h),
        (cache_read, rates.cache_read),
        (output, rates.output),
    ]
    .into_iter()
    .fold(0_u64, |sum, (tokens, rate)| {
        sum.saturating_add(tokens.saturating_mul(rate).saturating_mul(factor))
    });
    Cost {
        nanodollars,
        unpriced_requests: 0,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;

    /// The price table the plugin prices its live tally with.
    const SHARED_PRICES: &str = include_str!("../../fixtures/shared/tables/prices.json");

    fn rates_of_json(rates: &Value) -> Rates {
        let at = |key: &str| {
            rates[key]
                .as_u64()
                .unwrap_or_else(|| panic!("{key} in {rates}"))
        };
        Rates {
            input: at("input"),
            cache_write_5m: at("cache_write_5m"),
            cache_write_1h: at("cache_write_1h"),
            cache_read: at("cache_read"),
            output: at("output"),
        }
    }

    fn as_tuple(rates: Rates) -> (u64, u64, u64, u64, u64) {
        (
            rates.input,
            rates.cache_write_5m,
            rates.cache_write_1h,
            rates.cache_read,
            rates.output,
        )
    }

    // The plugin's live tally and the binary's cost of record must price a
    // token alike, or the spend gate refuses at another figure than rex shows.
    #[test]
    fn the_shared_price_table_is_the_one_the_binary_prices_with() {
        let shared: Value = serde_json::from_str(SHARED_PRICES).unwrap();
        for (model, rates) in shared["models"].as_object().unwrap() {
            let ours = rates_of(model, 1).unwrap_or_else(|| panic!("{model} has no price"));
            assert_eq!(as_tuple(ours), as_tuple(rates_of_json(rates)), "{model}");
        }
        for (model, card) in shared["second_card"].as_object().unwrap() {
            let above = card["above_prompt_tokens"].as_u64().unwrap();
            let ours = rates_of(model, above + 1).unwrap();
            assert_eq!(
                as_tuple(ours),
                as_tuple(rates_of_json(&card["rates"])),
                "{model}"
            );
            let below = rates_of(model, above).unwrap();
            assert_eq!(
                as_tuple(below),
                as_tuple(rates_of_json(&shared["models"][model])),
                "{model}"
            );
        }
        let fast: Vec<&str> = shared["fast_mode"]["models"]
            .as_array()
            .unwrap()
            .iter()
            .map(|model| model.as_str().unwrap())
            .collect();
        for model in shared["models"].as_object().unwrap().keys() {
            assert_eq!(
                has_fast_mode(model),
                fast.contains(&model.as_str()),
                "{model}"
            );
        }
        assert_eq!(shared["fast_mode"]["factor"], 2);
        assert_eq!(rates_of("claude-not-a-model", 1).map(as_tuple), None);
    }
}
