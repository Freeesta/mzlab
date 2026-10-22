//! The micro-dollar prices, the cost of a `usage` event and the meter that keeps a session under its cap. No network.

use mzlab_llm::event::Usage;
use mzlab_llm::spend::{Meter, Prices, DOLLAR};

fn usage(input: u64, output: u64) -> Usage {
    Usage {
        input,
        output,
        ..Usage::default()
    }
}

#[test]
fn cost_counts_every_kind_of_token_at_its_rate() {
    let p = Prices::list();
    // Sonnet 5.5: 2 / 10 / read 0.10 / write 2.50 and 4.00 USD per million tokens
    let u = Usage {
        input: 1000,
        output: 500,
        cache_read: 20_000,
        cache_write_5m: 100,
        cache_write_1h: 24_900,
    };
    assert_eq!(
        p.cost("claude-sonnet-5-5", &u).unwrap(),
        2000 + 5000 + 2000 + 250 + 99_600
    );
    // Opus 5.5: 4 / 20
    assert_eq!(
        p.cost("claude-opus-5-5", &usage(1000, 100)).unwrap(),
        4000 + 2000
    );
    // a fraction of a micro-dollar rounds up, never down to free
    assert_eq!(p.cost("claude-haiku-5-5", &usage(1, 0)).unwrap(), 1);
    assert_eq!(p.cost("claude-haiku-5-5", &usage(0, 0)).unwrap(), 0);
}

#[test]
fn haiku_costs_more_per_token_when_the_prompt_passes_100_000_tokens() {
    let p = Prices::list();
    let at = |input| p.cost("claude-haiku-5-5", &usage(input, 1000)).unwrap();
    assert_eq!(at(100_000), 10_000 + 500);
    assert_eq!(at(100_001), 50_000 + 2500 + 1); // 100 001 × 0.50 = 50 000.5, rounded up
                                                // the cached part counts toward the prompt size
    let u = Usage {
        input: 1,
        output: 0,
        cache_read: 100_000,
        ..Usage::default()
    };
    assert_eq!(p.cost("claude-haiku-5-5", &u).unwrap(), 1 + 5000);
}

#[test]
fn unknown_models_and_bad_tables_are_errors() {
    let p = Prices::list();
    assert_eq!(
        p.cost("nope", &usage(1, 1)).unwrap_err().key,
        "err.llm.prices"
    );
    // vendors with no verified price are not in the table: unknown never counts as free
    assert!(p.cost("gpt-4o-mini", &usage(1, 1)).is_err());
    assert!(Prices::parse("[]").is_err());
    assert!(Prices::parse(r#"{"m":{"in":-1,"out":1}}"#).is_err());
    assert!(Prices::parse(r#"{"m":{"in":1}}"#).is_err());
    let c = Prices::parse(r#"{"m":{"in":3,"out":15,"cache_read":0.3}}"#).unwrap();
    let u = Usage {
        input: 1000,
        output: 100,
        cache_read: 10_000,
        cache_write_5m: 1000,
        cache_write_1h: 1000,
    };
    // writes default to 1.25× and 2× the input price
    assert_eq!(c.cost("m", &u).unwrap(), 3000 + 1500 + 3000 + 3750 + 6000);
    let t =
        Prices::parse(r#"{"h":{"in":0.1,"out":0.5,"over":{"above_tokens":10,"in":1,"out":5}}}"#)
            .unwrap();
    assert_eq!(t.cost("h", &usage(10, 0)).unwrap(), 1);
    assert_eq!(t.cost("h", &usage(11, 0)).unwrap(), 11);
}

#[test]
fn the_meter_never_lets_the_worst_case_pass_the_cap() {
    let mut m = Meter::new(Prices::list(), 12_000);
    // Haiku, 10 000 estimated input tokens priced as a 1-hour cache write (0.20) + 1 000 output tokens (0.50) = 2 000 + 500 at worst
    let mut tickets = Vec::new();
    for _ in 0..4 {
        tickets.push(m.admit("claude-haiku-5-5", 10_000, 1000).unwrap());
    }
    let e = m.admit("claude-haiku-5-5", 10_000, 1000).unwrap_err();
    assert_eq!(e.key, "err.llm.limit.spend");
    assert_eq!(
        e.params,
        vec![("cap", "12000".to_string()), ("spent", "0".to_string())]
    );
    // a finished call replaces its reservation with its real cost and frees the difference
    let cost = m
        .charge(tickets.remove(0), "claude-haiku-5-5", &usage(10_000, 200))
        .unwrap();
    assert_eq!(cost, 1000 + 100);
    assert_eq!(m.spent(), 1100);
    m.admit("claude-haiku-5-5", 10_000, 1000).unwrap();
    assert!(m.admit("claude-haiku-5-5", 10_000, 1000).is_err());
    // a call that failed before any token gives its reservation back
    m.release(tickets.remove(0));
    m.admit("claude-haiku-5-5", 10_000, 1000).unwrap();
    assert!(m.admit("claude-haiku-5-5", 10_000, 1000).is_err());
}

#[test]
fn spent_plus_reserved_stays_under_the_cap_for_any_sequence() {
    let mut m = Meter::new(Prices::list(), 3 * DOLLAR / 100);
    let mut open = Vec::new();
    for i in 0..400u64 {
        let (est, max) = (500 + i * 37 % 4000, 100 + i * 91 % 900);
        if let Ok(t) = m.admit("claude-sonnet-5-5", est, max) {
            open.push((t, est, max));
        }
        if i % 3 == 0 {
            if let Some((t, est, max)) = open.pop() {
                // the real call stays within what it was admitted for
                m.charge(t, "claude-sonnet-5-5", &usage(est, max)).unwrap();
            }
        }
        assert!(m.spent() <= m.cap(), "round {i}");
    }
}

#[test]
fn the_soft_notice_comes_once_at_80_percent() {
    let mut m = Meter::new(Prices::list(), 10_000);
    let t = m.admit("claude-sonnet-5-5", 100, 100).unwrap();
    m.charge(t, "claude-sonnet-5-5", &usage(2000, 500)).unwrap(); // 4 000 + 5 000
    assert_eq!(m.spent(), 4000 + 5000);
    assert!(m.soft_notice());
    assert!(!m.soft_notice());
    let mut low = Meter::new(Prices::list(), 10_000);
    let t = low.admit("claude-sonnet-5-5", 100, 100).unwrap();
    low.charge(t, "claude-sonnet-5-5", &usage(1000, 500))
        .unwrap(); // 2 000 + 5 000 = 7 000
    assert!(!low.soft_notice());
}
