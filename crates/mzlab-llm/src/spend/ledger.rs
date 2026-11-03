//! The ledger of the public proxy (`proxy/assist-proxy.js`, `docs/agenti/assistente-proxy.md`): micro-dollar counters per session (`s:`), per IP hash
//! (`ip:`), per UTC day and per month, as a JSON state that the host keeps in one serialized store. `admit` reserves the worst case of a call before it
//! is forwarded and refuses it when any cap would pass; `settle` puts the true cost in place of the reservation; `rollover` starts a new day. Pure: no
//! clock, no randomness (the caller hands over the day and the entropy of the salt).

use mzlab_core::budget::sha256;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Out, Value};

use super::Prices;
use crate::event::Usage;
use crate::provider::anthropic::start_usage;

/// Spent and reserved micro-dollars of one counter.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Counter {
    pub spent: u64,
    pub held: u64,
}

impl Counter {
    fn total(&self) -> u64 {
        self.spent.saturating_add(self.held)
    }
}

/// The caps of one request in micro-dollars; `ip` 0 = the request has no IP counter (the owner in phase 0).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Caps {
    pub session: u64,
    pub ip: u64,
    pub day: u64,
    pub month: u64,
}

/// Who a request counts against: the random session id of the panel and the salted IP hash (`Ledger::ip_hash`), if counted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Keys {
    pub session: String,
    pub ip_hash: Option<String>,
}

/// A reservation, handed back to `settle` by the request that got it. It names the counters it holds money on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Hold {
    pub id: u64,
    pub model: String,
    pub amount: u64,
    pub day: String,
    pub keys: Vec<String>,
}

/// The state of the proxy: the UTC day and month it counts, the salt of the day (never leaves the ledger), the global sums and the counters of the day.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Ledger {
    pub day: String,
    pub month: String,
    salt: Option<[u8; 32]>,
    pub day_sum: Counter,
    pub month_sum: Counter,
    counters: Vec<(String, Counter)>,
    next: u64,
}

fn bad(field: &str) -> Error {
    Error::new("err.llm.ledger").with("field", field)
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn unhex32(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 {
        return None;
    }
    let mut out = [0u8; 32];
    for (i, o) in out.iter_mut().enumerate() {
        *o = u8::from_str_radix(s.get(2 * i..2 * i + 2)?, 16).ok()?;
    }
    Some(out)
}

/// HMAC-SHA256 (RFC 2104) of `msg` under a 32-byte key.
fn hmac(key: &[u8; 32], msg: &[u8]) -> [u8; 32] {
    let (mut inner, mut outer) = ([0x36u8; 64].to_vec(), [0x5cu8; 64].to_vec());
    for (i, k) in key.iter().enumerate() {
        inner[i] ^= k;
        outer[i] ^= k;
    }
    inner.extend_from_slice(msg);
    outer.extend_from_slice(&sha256(&inner));
    sha256(&outer)
}

fn is_day(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

/// A session id is the panel's random token: 8 to 64 of `[A-Za-z0-9_-]`, nothing that could carry a name.
fn is_session(s: &str) -> bool {
    (8..=64).contains(&s.len())
        && s.bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
}

fn counter_of(v: Option<&Value>) -> Result<Counter> {
    let n = |i| {
        v.and_then(|v| v.at(i))
            .and_then(Value::f64)
            .filter(|x| *x >= 0.0 && x.fract() == 0.0)
            .map(|x| x as u64)
    };
    match v {
        None => Ok(Counter::default()),
        Some(_) => Ok(Counter {
            spent: n(0).ok_or_else(|| bad("counter"))?,
            held: n(1).ok_or_else(|| bad("counter"))?,
        }),
    }
}

fn put_counter(o: &mut Out, c: &Counter) {
    o.raw("[")
        .int(c.spent as i64)
        .raw(",")
        .int(c.held as i64)
        .raw("]");
}

impl Ledger {
    /// The state as stored by the host; `""` is an empty ledger (the first request ever).
    pub fn parse(text: &str) -> Result<Ledger> {
        if text.trim().is_empty() {
            return Ok(Ledger::default());
        }
        let v = parse(text.as_bytes()).map_err(|_| bad("json"))?;
        if v.get("schema").and_then(Value::str) != Some("ledger/1") {
            return Err(bad("schema"));
        }
        let s = |k: &str| v.get(k).and_then(Value::str).unwrap_or("").to_string();
        let salt = match v.get("salt").and_then(Value::str) {
            None => None,
            Some(h) => Some(unhex32(h).ok_or_else(|| bad("salt"))?),
        };
        let mut counters = Vec::new();
        for (k, c) in v.get("c").map(Value::entries).unwrap_or(&[]) {
            counters.push((k.clone(), counter_of(Some(c))?));
        }
        Ok(Ledger {
            day: s("day"),
            month: s("month"),
            salt,
            day_sum: counter_of(v.get("g:day"))?,
            month_sum: counter_of(v.get("g:month"))?,
            counters,
            next: v.get("next").and_then(Value::f64).unwrap_or(0.0) as u64,
        })
    }

    pub fn to_json(&self) -> String {
        let mut o = Out::default();
        o.raw("{\"schema\":\"ledger/1\",\"day\":")
            .str(&self.day)
            .raw(",\"month\":")
            .str(&self.month)
            .raw(",\"next\":")
            .int(self.next as i64);
        if let Some(s) = &self.salt {
            o.raw(",\"salt\":").str(&hex(s));
        }
        o.raw(",\"g:day\":");
        put_counter(&mut o, &self.day_sum);
        o.raw(",\"g:month\":");
        put_counter(&mut o, &self.month_sum);
        o.raw(",\"c\":{");
        for (i, (k, c)) in self.counters.iter().enumerate() {
            if i > 0 {
                o.raw(",");
            }
            o.str(k).raw(":");
            put_counter(&mut o, c);
        }
        o.raw("}}");
        o.0
    }

    /// The counter of `key` (`s:<session>` or `ip:<hash>`), zero when the day has none.
    pub fn counter(&self, key: &str) -> Counter {
        self.counters
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, c)| *c)
            .unwrap_or_default()
    }

    fn slot(&mut self, key: &str) -> &mut Counter {
        let i = match self.counters.iter().position(|(k, _)| k == key) {
            Some(i) => i,
            None => {
                self.counters.push((key.to_string(), Counter::default()));
                self.counters.len() - 1
            }
        };
        &mut self.counters[i].1
    }

    /// Starts `day` (UTC, `YYYY-MM-DD`) when it is not the current one: every reservation still open becomes spending (a request cut at midnight is
    /// charged its worst case), the `s:` and `ip:` counters and the salt are dropped, the day sum restarts and the month sum restarts with a new month.
    pub fn rollover(&mut self, day: &str) -> Result<()> {
        if !is_day(day) {
            return Err(bad("day"));
        }
        if self.day == day {
            return Ok(());
        }
        self.month_sum.spent = self.month_sum.total();
        self.month_sum.held = 0;
        if self.month != day[..7] {
            self.month = day[..7].to_string();
            self.month_sum = Counter::default();
        }
        self.day = day.to_string();
        self.day_sum = Counter::default();
        self.counters.clear();
        self.salt = None;
        Ok(())
    }

    /// The key of an IP for today: HMAC-SHA256 under the salt of the day, cut to 16 bytes (32 hex digits). The first call of a day makes the salt from
    /// `entropy` (at least 16 random bytes of the caller); the IP itself is never stored.
    pub fn ip_hash(&mut self, ip: &str, entropy: &[u8]) -> Result<String> {
        let salt = match self.salt {
            Some(s) => s,
            None => {
                if entropy.len() < 16 {
                    return Err(bad("entropy"));
                }
                let s = sha256(entropy);
                self.salt = Some(s);
                s
            }
        };
        Ok(hex(&hmac(&salt, ip.as_bytes())[..16]))
    }

    /// Reserves the most the call can cost (`Prices::max_cost` of `ceil(in_bytes / 3)` prompt tokens and `max_tokens`) on the session, the IP, the
    /// day and the month. When spent + reserved + that worst case would pass any cap it is `err.llm.limit.spend` with `limit` = `session|ip|day|month`,
    /// `cap` and `spent`, and nothing is reserved.
    pub fn admit(
        &mut self,
        prices: &Prices,
        model: &str,
        in_bytes: u64,
        max_tokens: u64,
        keys: &Keys,
        caps: &Caps,
    ) -> Result<Hold> {
        if self.day.is_empty() {
            return Err(bad("day"));
        }
        if !is_session(&keys.session) {
            return Err(bad("session"));
        }
        let worst = prices.max_cost(model, in_bytes.div_ceil(3), max_tokens)?;
        let mut held = vec![format!("s:{}", keys.session)];
        let mut checks = vec![("session", self.counter(&held[0]), caps.session)];
        if let Some(h) = &keys.ip_hash {
            if h.len() != 32 || !h.bytes().all(|c| c.is_ascii_hexdigit()) {
                return Err(bad("ip_hash"));
            }
            if caps.ip > 0 {
                held.push(format!("ip:{h}"));
                checks.push(("ip", self.counter(&held[1]), caps.ip));
            }
        }
        checks.push(("day", self.day_sum, caps.day));
        checks.push(("month", self.month_sum, caps.month));
        for (limit, c, cap) in checks {
            if c.total().saturating_add(worst) > cap {
                return Err(Error::new("err.llm.limit.spend")
                    .with("limit", limit)
                    .with("cap", cap.to_string())
                    .with("spent", c.spent.to_string()));
            }
        }
        for k in &held {
            self.slot(k).held += worst;
        }
        self.day_sum.held += worst;
        self.month_sum.held += worst;
        self.next += 1;
        Ok(Hold {
            id: self.next,
            model: model.to_string(),
            amount: worst,
            day: self.day.clone(),
            keys: held,
        })
    }

    /// Closes a reservation: the cost of `usage` takes its place, or the whole reservation when the stream gave no complete `usage` (`None`). A ticket of
    /// an earlier day changes nothing: its reservation was already charged at the rollover. Returns the micro-dollars charged.
    pub fn settle(&mut self, prices: &Prices, t: &Hold, usage: Option<&Usage>) -> Result<u64> {
        let charge = match usage {
            Some(u) => prices.cost(&t.model, u)?,
            None => t.amount,
        };
        if t.day != self.day {
            return Ok(0);
        }
        let close = |c: &mut Counter| {
            c.held = c.held.saturating_sub(t.amount);
            c.spent = c.spent.saturating_add(charge);
        };
        for k in &t.keys {
            if let Some((_, c)) = self.counters.iter_mut().find(|(n, _)| n == k) {
                close(c);
            }
        }
        close(&mut self.day_sum);
        close(&mut self.month_sum);
        Ok(charge)
    }
}

impl Hold {
    pub fn to_json(&self) -> String {
        let mut o = Out::default();
        o.raw("{\"id\":")
            .int(self.id as i64)
            .raw(",\"model\":")
            .str(&self.model)
            .raw(",\"amount\":")
            .int(self.amount as i64)
            .raw(",\"day\":")
            .str(&self.day)
            .raw(",\"keys\":[");
        for (i, k) in self.keys.iter().enumerate() {
            if i > 0 {
                o.raw(",");
            }
            o.str(k);
        }
        o.raw("]}");
        o.0
    }

    pub fn parse(text: &str) -> Result<Hold> {
        let v = parse(text.as_bytes()).map_err(|_| bad("ticket"))?;
        let s = |k: &str| v.get(k).and_then(Value::str).map(String::from);
        let n = |k: &str| {
            v.get(k)
                .and_then(Value::f64)
                .filter(|x| *x >= 0.0)
                .map(|x| x as u64)
        };
        Ok(Hold {
            id: n("id").ok_or_else(|| bad("ticket"))?,
            model: s("model").ok_or_else(|| bad("ticket"))?,
            amount: n("amount").ok_or_else(|| bad("ticket"))?,
            day: s("day").ok_or_else(|| bad("ticket"))?,
            keys: v
                .get("keys")
                .map(Value::arr)
                .unwrap_or(&[])
                .iter()
                .filter_map(|k| k.str().map(String::from))
                .collect(),
        })
    }
}

impl Caps {
    /// `{"session": µ$, "ip": µ$, "day": µ$, "month": µ$}`; `ip` may be left out (0).
    pub fn parse(text: &str) -> Result<Caps> {
        let v = parse(text.as_bytes()).map_err(|_| bad("caps"))?;
        let n = |k: &str| {
            v.get(k)
                .and_then(Value::f64)
                .filter(|x| *x >= 0.0 && x.fract() == 0.0)
                .map(|x| x as u64)
        };
        Ok(Caps {
            session: n("session").ok_or_else(|| bad("caps"))?,
            ip: n("ip").unwrap_or(0),
            day: n("day").ok_or_else(|| bad("caps"))?,
            month: n("month").ok_or_else(|| bad("caps"))?,
        })
    }
}

impl Keys {
    /// `{"session": "...", "ip_hash": "32 hex" | null}`.
    pub fn parse(text: &str) -> Result<Keys> {
        let v = parse(text.as_bytes()).map_err(|_| bad("keys"))?;
        Ok(Keys {
            session: v
                .get("session")
                .and_then(Value::str)
                .ok_or_else(|| bad("session"))?
                .to_string(),
            ip_hash: v.get("ip_hash").and_then(Value::str).map(String::from),
        })
    }
}

/// The `usage` of an Anthropic stream as the proxy saw it: the `message.usage` of `message_start` and the `usage` of the last `message_delta`
/// (`{"start": {...}|null, "delta": {...}|null}`). `None` unless both arrived: a cut stream keeps its whole reservation.
pub fn usage_of_stream(text: &str) -> Result<Option<Usage>> {
    let v = parse(text.as_bytes()).map_err(|_| bad("usage"))?;
    let (Some(start), Some(delta)) = (
        v.get("start").filter(|x| !x.is_null()),
        v.get("delta").filter(|x| !x.is_null()),
    ) else {
        return Ok(None);
    };
    let mut u = start_usage(Some(start));
    let Some(out) = delta.get("output_tokens").and_then(Value::f64) else {
        return Ok(None);
    };
    u.output = out as u64;
    // newer streams repeat the prompt counts in message_delta: they win when present
    let n = |k: &str| delta.get(k).and_then(Value::f64).map(|x| x as u64);
    if let Some(x) = n("input_tokens") {
        u.input = x;
    }
    if let Some(x) = n("cache_read_input_tokens") {
        u.cache_read = x;
    }
    if delta.get("cache_creation").is_some() || n("cache_creation_input_tokens").is_some() {
        let w = start_usage(Some(delta));
        u.cache_write_5m = w.cache_write_5m;
        u.cache_write_1h = w.cache_write_1h;
    }
    Ok(Some(u))
}
