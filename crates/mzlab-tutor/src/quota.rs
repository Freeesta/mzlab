//! The caps of the tutor in integer micro-dollars (1 USD = 1 000 000): per student (money and questions a day), per IP fingerprint (questions a day), per day
//! for everyone and in total. `admit` decides before the model is called, `charge` counts the cost after, from the `usage` events and the prices of the
//! Worker variable `PRICES`. No float is stored, so counters never drift. Used by the Worker (state in Durable Objects) and by `eval` for the cost.
use mzlab_core::error::Error;
use mzlab_llm::event::Usage;
use mzlab_llm::spend;

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

/// The prices of the models, in the micro-dollars of `mzlab_llm::spend`; this wrapper only gives its errors the tutor key `err.tutor.prices`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Prices(spend::Prices);

fn tutor_error(e: Error) -> Error {
    e.params
        .into_iter()
        .fold(Error::new("err.tutor.prices"), |acc, (k, v)| acc.with(k, v))
}

impl Prices {
    /// The Worker variable `PRICES`: `{"model-id": {"in": 0.1, "out": 0.5, "cache_read": 0.01}}`, USD per million tokens (format of `spend::Prices::parse`).
    pub fn parse(text: &str) -> Result<Prices, Error> {
        spend::Prices::parse(text).map(Prices).map_err(tutor_error)
    }

    /// The prices shipped with the program (`data/fornitori-1.json`, checked on the date it says).
    pub fn list() -> Prices {
        Prices(spend::Prices::list())
    }

    /// Cost of one call in micro-dollars, rounded up. A model with no price is an error: unknown must never count as free.
    pub fn cost(&self, model: &str, u: &Usage) -> Result<u64, Error> {
        self.0.cost(model, u).map_err(tutor_error)
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
