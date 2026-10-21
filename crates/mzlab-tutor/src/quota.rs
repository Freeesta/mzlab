//! The caps of the tutor in integer micro-dollars (1 USD = 1 000 000): per student (money and questions a day), per IP fingerprint (questions a day), per day
//! for everyone and in total. `admit` decides before the model is called, `charge` counts the cost after, from the `usage` events and the prices of the
//! Worker variable `PRICES`. No float is stored, so counters never drift. Used by the Worker (state in Durable Objects) and by `eval` for the cost.
use mzlab_core::error::Error;
use mzlab_llm::event::Usage;
use serde_json::Value;

/// Micro-dollars in one cent.
pub const CENT: u64 = 10_000;
pub const DOLLAR: u64 = 100 * CENT;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Limits {
    pub student_total: u64,
    pub student_per_day: u32,
    pub ip_per_day: u32,
    pub global_total: u64,
    pub global_per_day: u64,
}

impl Limits {
    /// The recommended values: 1 USD and 25 questions a day per student, 60 questions a day per IP, 180 USD in total and 10 USD a day.
    pub fn recommended() -> Limits {
        Limits {
            student_total: DOLLAR,
            student_per_day: 25,
            ip_per_day: 60,
            global_total: 180 * DOLLAR,
            global_per_day: 10 * DOLLAR,
        }
    }
}

/// The counters one question looks at. The Worker fills it from the student, the IP fingerprint and the global objects; `day` is the day the three
/// day counters belong to (days since the epoch, or any number that changes every day).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct State {
    pub day: u32,
    pub student_spent: u64,
    pub student_today: u32,
    pub ip_today: u32,
    pub global_spent: u64,
    pub global_today: u64,
}

impl State {
    /// The same counters seen on `day`: the day counters restart at zero when the day changed.
    fn on(&self, day: u32) -> State {
        if self.day == day {
            return self.clone();
        }
        State {
            day,
            student_today: 0,
            ip_today: 0,
            global_today: 0,
            ..self.clone()
        }
    }
}

fn refuse(key: &'static str, limit: &'static str) -> Error {
    Error::new(key).with("limit", limit)
}

/// Whether one more question may go to the model. The global caps come first (they switch everyone to the passages mode), then the IP rate, then the
/// student. A counter equal to its cap refuses: the cap is the last value allowed to be reached, not exceeded.
pub fn admit(state: &State, limits: &Limits, now_day: u32) -> Result<(), Error> {
    let s = state.on(now_day);
    if s.global_spent >= limits.global_total {
        return Err(refuse("err.tutor.quota_global", "total"));
    }
    if s.global_today >= limits.global_per_day {
        return Err(refuse("err.tutor.quota_global", "day"));
    }
    if s.ip_today >= limits.ip_per_day {
        return Err(refuse("err.tutor.rate", "ip_day"));
    }
    if s.student_spent >= limits.student_total {
        return Err(refuse("err.tutor.quota_student", "total"));
    }
    if s.student_today >= limits.student_per_day {
        return Err(refuse("err.tutor.quota_student", "day"));
    }
    Ok(())
}

/// Price of a model in thousandths of a micro-dollar per token (= USD per million tokens × 1 000), so a price like 0.10 is exact.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ModelPrice {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prices(Vec<(String, ModelPrice)>);

impl Prices {
    /// The Worker variable `PRICES`: `{"model-id": {"in": 0.1, "out": 0.5, "cache_read": 0.01}}`, USD per million tokens (`cache_read` defaults to `in`).
    pub fn parse(text: &str) -> Result<Prices, Error> {
        let bad = |f: &str| Error::new("err.tutor.prices").with("field", f);
        let v: Value = serde_json::from_str(text).map_err(|_| bad("json"))?;
        let mut all = Vec::new();
        for (model, p) in v.as_object().ok_or_else(|| bad("json"))? {
            let get = |k: &str| match p.get(k) {
                None => Ok(None),
                Some(x) => x
                    .as_f64()
                    .filter(|x| x.is_finite() && *x >= 0.0)
                    .map(|x| Some((x * 1000.0).round() as u64))
                    .ok_or_else(|| bad(k)),
            };
            let input = get("in")?.ok_or_else(|| bad("in"))?;
            all.push((
                model.clone(),
                ModelPrice {
                    input,
                    output: get("out")?.ok_or_else(|| bad("out"))?,
                    cache_read: get("cache_read")?.unwrap_or(input),
                },
            ));
        }
        Ok(Prices(all))
    }

    /// The list prices the study assumes (to be checked at deploy time): Haiku 5.5 0.10/0.50, Sonnet 5.5 2/10 USD per million tokens.
    pub fn list() -> Prices {
        Prices::parse(
            r#"{"claude-haiku-5-5":{"in":0.1,"out":0.5,"cache_read":0.01},"claude-sonnet-5-5":{"in":2,"out":10,"cache_read":0.2}}"#,
        )
        .expect("the list prices are valid")
    }

    /// Cost of one call in micro-dollars, rounded up. A model with no price is an error: unknown must never count as free.
    pub fn cost(&self, model: &str, u: &Usage) -> Result<u64, Error> {
        let p = self
            .0
            .iter()
            .find(|(m, _)| m == model)
            .map(|(_, p)| p)
            .ok_or_else(|| Error::new("err.tutor.prices").with("model", model))?;
        let thousandths = u128::from(u.input) * u128::from(p.input)
            + u128::from(u.output) * u128::from(p.output)
            + u128::from(u.cache_read) * u128::from(p.cache_read);
        Ok(thousandths.div_ceil(1000) as u64)
    }
}

/// Counts one question that went to the model: the cost of each `(model, usage)` call is added to the money counters and the question to the day
/// counters (a day change restarts them first). Returns the cost in micro-dollars.
pub fn charge(
    state: &mut State,
    calls: &[(&str, Usage)],
    prices: &Prices,
    now_day: u32,
) -> Result<u64, Error> {
    let mut cost = 0u64;
    for (model, usage) in calls {
        cost = cost.saturating_add(prices.cost(model, usage)?);
    }
    let mut s = state.on(now_day);
    s.student_spent = s.student_spent.saturating_add(cost);
    s.global_spent = s.global_spent.saturating_add(cost);
    s.global_today = s.global_today.saturating_add(cost);
    s.student_today += 1;
    s.ip_today += 1;
    *state = s;
    Ok(cost)
}
