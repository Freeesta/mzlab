//! Money in integer micro-dollars (1 USD = 1 000 000): the prices per million tokens, the cost of a `usage` event (cache reads and the two cache writes at
//! their own rates, Haiku's higher tier above 100 000 prompt tokens) and the `Meter` that decides before each call whether the cap still allows it.
//! No float is stored, so counters never drift. Used by the assistant, `mzlab-cli` and, through `mzlab-tutor::quota`, by the tutor Worker.

use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Value};

use crate::event::Usage;
use crate::provider::{Price, Provider};

/// Micro-dollars in one dollar.
pub const DOLLAR: u64 = 1_000_000;
/// The share of the cap (percent) after which the assistant says once that the budget is running out.
pub const SOFT_PERCENT: u64 = 80;

/// Micro-dollars per million tokens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rates {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub write_5m: u64,
    pub write_1h: u64,
}

/// The rates of a model; `over` = the rates that apply to the whole request when its prompt has more than that many tokens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelPrice {
    pub base: Rates,
    pub over: Option<(u64, Rates)>,
}

impl ModelPrice {
    fn rates(&self, prompt_tokens: u64) -> &Rates {
        match &self.over {
            Some((above, r)) if prompt_tokens > *above => r,
            _ => &self.base,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prices(Vec<(String, ModelPrice)>);

fn bad(field: &str) -> Error {
    Error::new("err.llm.prices").with("field", field)
}

/// USD per million tokens to micro-dollars per million tokens.
fn micro(usd: f64) -> u64 {
    (usd * 1e6).round() as u64
}

fn rates_of(p: &Price) -> Rates {
    Rates {
        input: micro(p.input),
        output: micro(p.output),
        cache_read: micro(p.cache_read),
        write_5m: micro(p.write_5m),
        write_1h: micro(p.write_1h),
    }
}

impl Prices {
    /// The models of the providers that have a price (input and output above 0); unknown prices are left out, so they never count as free.
    pub fn from_providers(all: &[Provider]) -> Prices {
        let mut v = Vec::new();
        for m in all.iter().flat_map(|p| &p.models) {
            if m.price.input > 0.0 && m.price.output > 0.0 {
                let over = m
                    .price
                    .over
                    .as_ref()
                    .map(|o| (o.above_tokens, rates_of(&o.rates)));
                v.push((
                    m.id.clone(),
                    ModelPrice {
                        base: rates_of(&m.price),
                        over,
                    },
                ));
            }
        }
        Prices(v)
    }

    /// The prices shipped in `data/fornitori-1.json`.
    pub fn list() -> Prices {
        Prices::from_providers(&crate::provider::builtin())
    }

    /// `{"model-id": {"in": 0.1, "out": 0.5, "cache_read": 0.01, "write_5m": 0.125, "write_1h": 0.2, "over": {"above_tokens": 100000, "in": ..}}}`,
    /// USD per million tokens. `cache_read` defaults to `in`, the writes to 1.25× and 2× `in` (Anthropic's multipliers); `over` repeats the same keys.
    pub fn parse(text: &str) -> Result<Prices> {
        let v = parse(text.as_bytes()).map_err(|_| bad("json"))?;
        let Value::Obj(models) = &v else {
            return Err(bad("json"));
        };
        let mut all = Vec::new();
        for (model, p) in models {
            let base = rates_from(p)?;
            let over = match p.get("over") {
                None => None,
                Some(o) => {
                    let above = o
                        .get("above_tokens")
                        .and_then(Value::f64)
                        .filter(|x| *x >= 0.0)
                        .ok_or_else(|| bad("above_tokens"))?;
                    Some((above as u64, rates_from(o)?))
                }
            };
            all.push((model.clone(), ModelPrice { base, over }));
        }
        Ok(Prices(all))
    }

    fn get(&self, model: &str) -> Result<&ModelPrice> {
        self.0
            .iter()
            .find(|(m, _)| m == model)
            .map(|(_, p)| p)
            .ok_or_else(|| bad("model").with("model", model))
    }

    /// Cost of one call in micro-dollars, rounded up. A model with no price is an error: unknown must never count as free.
    pub fn cost(&self, model: &str, u: &Usage) -> Result<u64> {
        let p = self.get(model)?;
        let prompt = u.input + u.cache_read + u.cache_write_5m + u.cache_write_1h;
        let r = p.rates(prompt);
        let total = u128::from(u.input) * u128::from(r.input)
            + u128::from(u.output) * u128::from(r.output)
            + u128::from(u.cache_read) * u128::from(r.cache_read)
            + u128::from(u.cache_write_5m) * u128::from(r.write_5m)
            + u128::from(u.cache_write_1h) * u128::from(r.write_1h);
        Ok(total.div_ceil(1_000_000) as u64)
    }

    /// The most a call can cost: the estimated prompt at the dearest input-side rate (it may turn out to be a cache write) plus `max_tokens` at the output rate.
    pub fn max_cost(&self, model: &str, est_input: u64, max_tokens: u64) -> Result<u64> {
        let p = self.get(model)?;
        let r = p.rates(est_input);
        let input = r.input.max(r.write_5m).max(r.write_1h);
        let total = u128::from(est_input) * u128::from(input)
            + u128::from(max_tokens) * u128::from(r.output);
        Ok(total.div_ceil(1_000_000) as u64)
    }
}

fn rates_from(p: &Value) -> Result<Rates> {
    let get = |k: &str| match p.get(k) {
        None => Ok(None),
        Some(x) => x
            .f64()
            .filter(|x| x.is_finite() && *x >= 0.0)
            .map(|x| Some(micro(x)))
            .ok_or_else(|| bad(k)),
    };
    let input = get("in")?.ok_or_else(|| bad("in"))?;
    Ok(Rates {
        input,
        output: get("out")?.ok_or_else(|| bad("out"))?,
        cache_read: get("cache_read")?.unwrap_or(input),
        write_5m: get("write_5m")?.unwrap_or(input * 5 / 4),
        write_1h: get("write_1h")?.unwrap_or(input * 2),
    })
}

/// A reservation made by `Meter::admit`, given back to `charge` or `release`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ticket(u64);

/// The spend of one session against its cap. `admit` reserves the worst case of a call before it is sent; `charge` replaces the reservation with the cost
/// of the `usage` events. The reserved money counts against the cap, so calls in flight together cannot pass it either.
#[derive(Clone, Debug)]
pub struct Meter {
    prices: Prices,
    cap: u64,
    spent: u64,
    pending: u64,
    soft_told: bool,
}

impl Meter {
    pub fn new(prices: Prices, cap: u64) -> Meter {
        Meter {
            prices,
            cap,
            spent: 0,
            pending: 0,
            soft_told: false,
        }
    }

    /// May a call with this estimated prompt and `max_tokens` go out? It never lets `spent + reserved + worst case` pass the cap
    /// (`err.llm.limit.spend`, with `cap` and `spent` in micro-dollars).
    pub fn admit(&mut self, model: &str, est_input: u64, max_tokens: u64) -> Result<Ticket> {
        let worst = self.prices.max_cost(model, est_input, max_tokens)?;
        if self
            .spent
            .saturating_add(self.pending)
            .saturating_add(worst)
            > self.cap
        {
            return Err(Error::new("err.llm.limit.spend")
                .with("cap", self.cap.to_string())
                .with("spent", self.spent.to_string()));
        }
        self.pending += worst;
        Ok(Ticket(worst))
    }

    /// Counts the `usage` of the call and lifts its reservation; returns the cost in micro-dollars.
    pub fn charge(&mut self, t: Ticket, model: &str, u: &Usage) -> Result<u64> {
        let cost = self.prices.cost(model, u)?;
        self.release(t);
        self.spent = self.spent.saturating_add(cost);
        Ok(cost)
    }

    /// Lifts the reservation of a call that produced no `usage` (failed before the first token).
    pub fn release(&mut self, t: Ticket) {
        self.pending = self.pending.saturating_sub(t.0);
    }

    pub fn spent(&self) -> u64 {
        self.spent
    }

    pub fn cap(&self) -> u64 {
        self.cap
    }

    /// True once, the first time the spend reaches `SOFT_PERCENT` of the cap; the assistant says it and offers a summary.
    pub fn soft_notice(&mut self) -> bool {
        if self.soft_told || self.spent * 100 < self.cap * SOFT_PERCENT {
            return false;
        }
        self.soft_told = true;
        true
    }
}
