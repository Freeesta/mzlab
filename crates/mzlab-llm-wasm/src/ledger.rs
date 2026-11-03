//! The ledger of the public proxy (`proxy/assist-proxy.js`) through `mzlab_llm::spend::Ledger`: the host keeps the JSON state, each call takes it and
//! gives back `{"...": result, "state": new state}`. The same code counts the money in the CLI, the desktop app, the site and the proxy.

use std::sync::OnceLock;

use mzlab_llm::spend::{usage_of_stream, Caps, Hold, Keys, Ledger, Prices};
use wasm_bindgen::prelude::*;

use crate::fail;

fn prices() -> &'static Prices {
    static P: OnceLock<Prices> = OnceLock::new();
    P.get_or_init(Prices::list)
}

fn load(state: &str) -> Result<Ledger, String> {
    Ledger::parse(state).map_err(fail)
}

fn session_of(l: &Ledger, h: &Hold) -> String {
    let c = h.keys.first().map(|k| l.counter(k)).unwrap_or_default();
    format!("[{},{}]", c.spent, c.held)
}

/// The state after `day` (UTC `YYYY-MM-DD`) began: open reservations charged, `s:`/`ip:` counters and salt dropped (`Ledger::rollover`).
#[wasm_bindgen]
pub fn ledger_rollover(state: &str, day: &str) -> Result<String, String> {
    let mut l = load(state)?;
    l.rollover(day).map_err(fail)?;
    Ok(l.to_json())
}

/// `{"ip_hash": 32 hex, "state"}`: the salted hash of `ip` for today; `entropy` (≥ 16 random bytes) makes the salt on the first call of a day.
#[wasm_bindgen]
pub fn ledger_ip_hash(state: &str, ip: &str, entropy: &[u8]) -> Result<String, String> {
    let mut l = load(state)?;
    let h = l.ip_hash(ip, entropy).map_err(fail)?;
    Ok(format!("{{\"ip_hash\":\"{h}\",\"state\":{}}}", l.to_json()))
}

/// `{"ticket", "session": [spent, held], "state"}`, or the `err.llm.limit.spend` of the first cap that would pass (`Ledger::admit`).
#[wasm_bindgen]
pub fn ledger_admit(
    state: &str,
    model: &str,
    in_bytes: u32,
    max_tokens: u32,
    keys_json: &str,
    caps_json: &str,
) -> Result<String, String> {
    let mut l = load(state)?;
    let keys = Keys::parse(keys_json).map_err(fail)?;
    let caps = Caps::parse(caps_json).map_err(fail)?;
    let t = l
        .admit(
            prices(),
            model,
            u64::from(in_bytes),
            u64::from(max_tokens),
            &keys,
            &caps,
        )
        .map_err(fail)?;
    Ok(format!(
        "{{\"ticket\":{},\"session\":{},\"state\":{}}}",
        t.to_json(),
        session_of(&l, &t),
        l.to_json()
    ))
}

/// `{"charged", "session": [spent, held], "state"}`: `usage_json` is `{"start": message_start usage, "delta": last message_delta usage}`, or `""`
/// when the stream gave none (the whole reservation stays).
#[wasm_bindgen]
pub fn ledger_settle(state: &str, ticket_json: &str, usage_json: &str) -> Result<String, String> {
    let mut l = load(state)?;
    let t = Hold::parse(ticket_json).map_err(fail)?;
    let u = match usage_json.trim() {
        "" => None,
        s => usage_of_stream(s).map_err(fail)?,
    };
    let charged = l.settle(prices(), &t, u.as_ref()).map_err(fail)?;
    Ok(format!(
        "{{\"charged\":{charged},\"session\":{},\"state\":{}}}",
        session_of(&l, &t),
        l.to_json()
    ))
}

/// The most a call can cost in micro-dollars (`Prices::max_cost`).
#[wasm_bindgen]
pub fn max_cost(model: &str, est_input: u32, max_tokens: u32) -> Result<f64, String> {
    prices()
        .max_cost(model, u64::from(est_input), u64::from(max_tokens))
        .map(|v| v as f64)
        .map_err(fail)
}

/// `[[name, schema], ...]`: the closed answer schemas a request may force, for the proxy allow-list.
#[wasm_bindgen]
pub fn assist_schemas() -> String {
    let mut o = mzlab_core::json::Out::default();
    o.raw("[");
    for (i, (n, s)) in mzlab_llm::subcheck::closed_schemas().iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("[")
            .str(n)
            .raw(",")
            .str(s)
            .raw("]");
    }
    o.raw("]");
    o.0
}
