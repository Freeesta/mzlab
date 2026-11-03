//! The micro-dollar prices, the cost of a `usage` event and the meter that keeps a session under its cap. No network.

use mzlab_llm::event::Usage;
use mzlab_llm::spend::{usage_of_stream, Caps, Keys, Ledger, Meter, Prices, DOLLAR};

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
        if i % 3 == 0
            && let Some((t, est, max)) = open.pop()
        {
            // the real call stays within what it was admitted for
            m.charge(t, "claude-sonnet-5-5", &usage(est, max)).unwrap();
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

// ---- the ledger of the public proxy

const SONNET: &str = "claude-sonnet-5-5";

fn caps() -> Caps {
    Caps {
        session: DOLLAR / 2,
        ip: DOLLAR,
        day: 6 * DOLLAR,
        month: 150 * DOLLAR,
    }
}

fn keys(session: &str, ip: Option<&str>) -> Keys {
    Keys {
        session: session.to_string(),
        ip_hash: ip.map(String::from),
    }
}

fn ledger() -> Ledger {
    let mut l = Ledger::default();
    l.rollover("2026-11-03").unwrap();
    l
}

const IP1: &str = "00112233445566778899aabbccddeeff";

#[test]
fn ledger_never_lets_any_cap_pass_and_names_the_one_that_stops() {
    let p = Prices::list();
    let mut l = ledger();
    // 3 000 bytes = 1 000 prompt tokens at 4.00 (the dearest input-side rate) + 2 000 output tokens at 10.00 = 24 000 µ$
    let worst = p.max_cost(SONNET, 1000, 2000).unwrap();
    assert_eq!(worst, 24_000);
    let k = keys("sessionAAAA", Some(IP1));
    let mut admitted = 0;
    let err = loop {
        match l.admit(&p, SONNET, 3000, 2000, &k, &caps()) {
            Ok(t) => {
                admitted += 1;
                l.settle(&p, &t, None).unwrap();
            }
            Err(e) => break e,
        }
    };
    assert_eq!(admitted, 20); // 20 × 24 000 = 480 000 ≤ 500 000 < 21 × 24 000
    assert_eq!(err.key, "err.llm.limit.spend");
    let param = |n: &str| err.params.iter().find(|(k, _)| *k == n).unwrap().1.clone();
    assert_eq!(param("limit"), "session");
    assert_eq!(param("cap"), "500000");
    assert_eq!(l.counter("s:sessionAAAA").spent, 480_000);
    // new sessions of the same IP stop at the IP cap (1 $), then the day cap stops other IPs
    let mut spent_ip = 480_000;
    let mut s = 0;
    let err = loop {
        s += 1;
        match l.admit(
            &p,
            SONNET,
            3000,
            2000,
            &keys(&format!("session{s:04}"), Some(IP1)),
            &caps(),
        ) {
            Ok(t) => {
                spent_ip += l.settle(&p, &t, None).unwrap();
            }
            Err(e) => break e,
        }
    };
    assert!(spent_ip <= DOLLAR);
    assert_eq!(err.params[0], ("limit", "ip".to_string()));
    for c in [l.counter(&format!("ip:{IP1}")), l.day_sum, l.month_sum] {
        assert!(c.spent + c.held <= DOLLAR);
    }
    let small = Caps {
        day: 1_000_000,
        ..caps()
    };
    let e = l
        .admit(
            &p,
            SONNET,
            3000,
            2000,
            &keys("otherIPsession", Some("ffeeddccbbaa99887766554433221100")),
            &small,
        )
        .unwrap_err();
    assert_eq!(e.params[0], ("limit", "day".to_string()));
    let tiny = Caps {
        month: 10,
        ..caps()
    };
    let e = l
        .admit(&p, SONNET, 3, 1, &keys("monthsession", None), &tiny)
        .unwrap_err();
    assert_eq!(e.params[0], ("limit", "month".to_string()));
}

#[test]
fn ledger_two_parallel_admits_cannot_pass_the_session_cap_together() {
    let p = Prices::list();
    let mut l = ledger();
    let k = keys("parallelSess", Some(IP1));
    // worst case 10 000 prompt tokens × 4 + 30 000 output tokens × 10 = 340 000 µ$: one fits under 0.50 $, two in flight together would not
    let a = l.admit(&p, SONNET, 30_000, 30_000, &k, &caps()).unwrap();
    assert_eq!(a.amount, 340_000);
    let b = l
        .admit(&p, SONNET, 30_000, 30_000, &k, &caps())
        .unwrap_err();
    assert_eq!(b.params[0], ("limit", "session".to_string()));
    let c = l.counter("s:parallelSess");
    assert_eq!((c.spent, c.held), (0, 340_000));
    // the true cost replaces the reservation, then the second one fits
    let u = Usage {
        input: 1000,
        output: 100,
        ..Usage::default()
    };
    assert_eq!(l.settle(&p, &a, Some(&u)).unwrap(), 2000 + 1000);
    let b = l.admit(&p, SONNET, 30_000, 30_000, &k, &caps()).unwrap();
    l.settle(&p, &b, Some(&u)).unwrap();
    let c = l.counter("s:parallelSess");
    assert_eq!((c.spent, c.held), (6000, 0));
    assert_eq!((l.day_sum.spent, l.day_sum.held), (6000, 0));
    assert_eq!(l.counter(&format!("ip:{IP1}")).spent, 6000);
}

#[test]
fn ledger_a_truncated_stream_keeps_the_whole_reservation() {
    let p = Prices::list();
    let mut l = ledger();
    let t = l
        .admit(&p, SONNET, 3000, 2000, &keys("truncated01", None), &caps())
        .unwrap();
    let start = r#"{"start":{"input_tokens":10,"output_tokens":1},"delta":null}"#;
    assert_eq!(usage_of_stream(start).unwrap(), None);
    assert_eq!(l.settle(&p, &t, None).unwrap(), t.amount);
    assert_eq!(l.counter("s:truncated01").spent, 24_000);
    // a complete stream: the prompt of message_start, the output of the last message_delta, the cache detail of message_start
    let full = r#"{"start":{"input_tokens":12,"output_tokens":1,"cache_creation_input_tokens":25000,"cache_read_input_tokens":0,"cache_creation":{"ephemeral_5m_input_tokens":100,"ephemeral_1h_input_tokens":24900}},"delta":{"output_tokens":4}}"#;
    let u = usage_of_stream(full).unwrap().unwrap();
    assert_eq!(
        (u.input, u.output, u.cache_write_5m, u.cache_write_1h),
        (12, 4, 100, 24_900)
    );
}

#[test]
fn ledger_rollover_drops_session_and_ip_counters_and_keeps_the_month() {
    let p = Prices::list();
    let mut l = ledger();
    let ip = l.ip_hash("203.0.113.7", &[7u8; 32]).unwrap();
    assert_eq!(ip.len(), 32);
    assert_eq!(
        l.ip_hash("203.0.113.7", &[]).unwrap(),
        ip,
        "same salt all day"
    );
    assert_ne!(l.ip_hash("203.0.113.8", &[]).unwrap(), ip);
    let done = l
        .admit(
            &p,
            SONNET,
            3000,
            2000,
            &keys("rolloverSess", Some(&ip)),
            &caps(),
        )
        .unwrap();
    l.settle(&p, &done, None).unwrap();
    let open = l
        .admit(
            &p,
            SONNET,
            3000,
            2000,
            &keys("rolloverSess", Some(&ip)),
            &caps(),
        )
        .unwrap();
    let text = l.to_json();
    assert!(!text.contains("203.0.113"), "the IP is never stored");
    assert_eq!(Ledger::parse(&text).unwrap(), l);
    l.rollover("2026-11-04").unwrap();
    assert_eq!(l.counter("s:rolloverSess").spent, 0);
    assert_eq!(l.counter(&format!("ip:{ip}")).spent, 0);
    assert_eq!(l.day_sum.spent, 0);
    // the reservation open at midnight was charged whole to the month; settling it later changes nothing
    assert_eq!(l.month_sum.spent, 48_000);
    l.settle(&p, &open, Some(&Usage::default())).unwrap();
    assert_eq!((l.month_sum.spent, l.month_sum.held), (48_000, 0));
    // a new salt: yesterday's hash of the same IP is not today's, and today needs fresh entropy
    assert!(l.ip_hash("203.0.113.7", &[]).is_err());
    assert_ne!(l.ip_hash("203.0.113.7", &[9u8; 32]).unwrap(), ip);
    // a new month starts the month sum again
    l.rollover("2026-12-01").unwrap();
    assert_eq!(l.month_sum.spent, 0);
    assert!(l.rollover("tomorrow").is_err());
}

#[test]
fn ledger_ip_hash_is_hmac_sha256_cut_to_16_bytes() {
    // the key is SHA-256(entropy); the value is Python's hmac.new(sha256(b"0123456789abcdef"), b"203.0.113.7", sha256), first 16 bytes
    let mut l = ledger();
    let h = l.ip_hash("203.0.113.7", b"0123456789abcdef").unwrap();
    assert_eq!(h, "db3fa6e697d1d8115e85146bf4fe5609");
    assert!(Ledger::parse("{\"schema\":\"other\"}").is_err());
    let e = l.admit(
        &Prices::list(),
        SONNET,
        1,
        1,
        &keys("bad id!", None),
        &caps(),
    );
    assert_eq!(e.unwrap_err().key, "err.llm.ledger");
}
