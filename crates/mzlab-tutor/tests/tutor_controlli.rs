//! The checks on an answer, the caps and the charge in micro-dollars, and the level-b cleaning of the statistics, on the synthetic corpus
//! `tests/data/tutor/blocchi-mini.json`. No network, no key.
use mzlab_llm::event::Usage;
use mzlab_tutor::answer;
use mzlab_tutor::checks;
use mzlab_tutor::eval::Corpus;
use mzlab_tutor::pick::Intent;
use mzlab_tutor::quota::{admit, charge, Limits, Prices, State, CENT, DOLLAR};
use mzlab_tutor::stat::{share_text, Record, Stat};
use std::{fs, path::PathBuf};

fn data(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/data/tutor")
        .join(name)
}

fn corpus() -> Corpus {
    Corpus::parse(&fs::read_to_string(data("blocchi-mini.json")).unwrap()).unwrap()
}

fn same_as_file(name: &str, body: &str) {
    let path = data(name);
    if std::env::var("MZLAB_UPDATE_TUTOR").is_ok() {
        fs::write(&path, format!("{body}\n")).unwrap();
    }
    assert_eq!(
        format!("{body}\n"),
        fs::read_to_string(&path).unwrap(),
        "{name} differs (MZLAB_UPDATE_TUTOR=1 rewrites it)"
    );
}

fn sent() -> Vec<answer::Block> {
    answer::select(
        &corpus().value,
        &[
            "09-quadrupolo#stabilita.1".to_string(),
            "12-dati#metodo.1".to_string(),
        ],
    )
}

#[test]
fn a_good_answer_passes_every_check() {
    let c = corpus();
    let a = "Uno ione è trasmesso se a e q cadono nella regione di stabilità [1]. Con a = 0 il limite è q = 0,908 [1]. Dove cadrebbe il vostro ione?";
    let k = checks::run(a, &sent(), &c.hidden);
    assert!(k.cites_ok && k.numbers_ok && k.solution_ok(), "{k:?}");
    assert!(k.bad_sentences.is_empty());
}

#[test]
fn an_invented_citation_is_flagged() {
    let a = "Uno ione è trasmesso se cade nella regione di stabilità [1]. Lo dice anche il capitolo sul vuoto [5].";
    let k = checks::run(a, &sent(), &[]);
    assert!(!k.cites_ok);
    assert_eq!(k.bad_sentences, vec![1]);
    let none = checks::run(
        "Uno ione è trasmesso se cade nella regione di stabilità.",
        &sent(),
        &[],
    );
    assert!(!none.cites_ok && none.bad_sentences == vec![0]);
    // a question and the fixed sentence need no citation; a bracketed ion is not a citation
    let free = checks::run(
        "La Teoria non tratta questo punto. Che cosa ne pensate di [M+H]+?",
        &sent(),
        &[],
    );
    assert!(free.cites_ok, "{free:?}");
}

#[test]
fn an_invented_number_is_flagged() {
    let a = "Con a = 0 il limite è q = 0,908 [1]. Con la risoluzione unitaria la finestra è larga 2,5 Da [1].";
    let k = checks::run(a, &sent(), &[]);
    assert!(k.cites_ok);
    assert!(!k.numbers_ok);
    assert_eq!(k.bad_sentences, vec![1]);
    // a number of another sent block does not count for a sentence that cites a different one
    let b = checks::run("Il limite è q = 0,908 [2].", &sent(), &[]);
    assert!(!b.numbers_ok);
    // decimal comma and point are the same number
    assert!(checks::run("Il limite è q = 0.908 [1].", &sent(), &[]).numbers_ok);
}

#[test]
fn a_copied_hidden_answer_is_flagged() {
    let c = corpus();
    let hidden = c.value["blocks"][2]["hidden_shingles"]
        .as_array()
        .unwrap()
        .len();
    assert!(hidden > 10);
    let copy = "No: il limite per a = 0 è q = 0,908 e 0,95 sta oltre, quindi la traiettoria cresce fino alle barre.";
    let k = checks::run(copy, &sent(), &c.hidden);
    assert!(k.solution_overlap > 0.9 && !k.solution_ok(), "{k:?}");
    let part = "Il vostro ione: il limite per a = 0 è q = 0,908 [1].";
    let k = checks::run(part, &sent(), &c.hidden);
    assert!(
        k.solution_overlap > 0.0 && k.solution_overlap < 0.5,
        "{k:?}"
    );
}

fn usage(i: u64, o: u64) -> Usage {
    Usage {
        input: i,
        output: o,
        ..Usage::default()
    }
}

#[test]
fn caps_at_the_edge() {
    let l = Limits::recommended();
    let day = 20_000;
    let s = State {
        day,
        ..State::default()
    };
    assert!(admit(&s, &l, day).is_ok());
    // one cent under the student cap is allowed, the cap itself is not
    let s = State {
        day,
        student_spent: l.student_total - 1,
        ..State::default()
    };
    assert!(admit(&s, &l, day).is_ok());
    let s = State {
        day,
        student_spent: l.student_total,
        ..State::default()
    };
    assert_eq!(
        admit(&s, &l, day).unwrap_err().key,
        "err.tutor.quota_student"
    );
    let s = State {
        day,
        student_today: 24,
        ..State::default()
    };
    assert!(admit(&s, &l, day).is_ok());
    let s = State {
        day,
        student_today: 25,
        ..State::default()
    };
    assert_eq!(
        admit(&s, &l, day).unwrap_err().key,
        "err.tutor.quota_student"
    );
    // the day counters restart on a new day, the money does not
    assert!(admit(&s, &l, day + 1).is_ok());
    let rich = State {
        day,
        student_today: 25,
        student_spent: l.student_total,
        ..State::default()
    };
    assert_eq!(
        admit(&rich, &l, day + 1).unwrap_err().key,
        "err.tutor.quota_student"
    );
    let ip = State {
        day,
        ip_today: 60,
        ..State::default()
    };
    assert_eq!(admit(&ip, &l, day).unwrap_err().key, "err.tutor.rate");
    assert!(admit(&ip, &l, day + 1).is_ok());
    let g = State {
        day,
        global_today: l.global_per_day,
        ..State::default()
    };
    assert_eq!(
        admit(&g, &l, day).unwrap_err().key,
        "err.tutor.quota_global"
    );
    assert!(admit(&g, &l, day + 1).is_ok());
}

#[test]
fn the_global_cap_is_seen_before_the_student_one() {
    let l = Limits::recommended();
    let s = State {
        day: 1,
        student_spent: l.student_total,
        student_today: 99,
        ip_today: 99,
        global_spent: l.global_total,
        global_today: 0,
    };
    let e = admit(&s, &l, 1).unwrap_err();
    assert_eq!(e.key, "err.tutor.quota_global");
    assert_eq!(e.params, vec![("limit", "total".to_string())]);
}

#[test]
fn charge_counts_in_integer_micro_dollars() {
    let p = Prices::list();
    // Haiku 2 500 in / 100 out = 250 + 50 = 300; Sonnet 3 200 / 800 = 6 400 + 8 000 = 14 400
    assert_eq!(p.cost("claude-haiku-5-5", &usage(2500, 100)).unwrap(), 300);
    assert_eq!(
        p.cost("claude-sonnet-5-5", &usage(3200, 800)).unwrap(),
        14_400
    );
    // a fraction of a micro-dollar rounds up, never down to free
    assert_eq!(p.cost("claude-haiku-5-5", &usage(1, 0)).unwrap(), 1);
    assert_eq!(p.cost("claude-haiku-5-5", &usage(0, 0)).unwrap(), 0);
    assert_eq!(
        p.cost("nope", &usage(1, 1)).unwrap_err().key,
        "err.tutor.prices"
    );
    let mut s = State {
        day: 5,
        ..State::default()
    };
    let cost = charge(
        &mut s,
        &[
            ("claude-haiku-5-5", usage(2500, 100)),
            ("claude-sonnet-5-5", usage(3200, 800)),
        ],
        &p,
        5,
    )
    .unwrap();
    assert_eq!(cost, 14_700);
    assert_eq!(
        (
            s.student_spent,
            s.global_spent,
            s.global_today,
            s.student_today,
            s.ip_today
        ),
        (14_700, 14_700, 14_700, 1, 1)
    );
    // the next day the day counters start again from this question
    charge(&mut s, &[("claude-haiku-5-5", usage(2500, 100))], &p, 6).unwrap();
    assert_eq!(
        (s.day, s.student_spent, s.global_today, s.student_today),
        (6, 15_000, 300, 1)
    );
    assert_eq!(DOLLAR, 100 * CENT);
    let custom = Prices::parse(r#"{"m":{"in":3,"out":15,"cache_read":0.3}}"#).unwrap();
    assert_eq!(
        custom
            .cost(
                "m",
                &Usage {
                    input: 1000,
                    output: 100,
                    cache_read: 10_000,
                    ..Usage::default()
                }
            )
            .unwrap(),
        3000 + 1500 + 3000
    );
    assert!(Prices::parse(r#"{"m":{"in":-1,"out":1}}"#).is_err());
}

#[test]
fn share_text_removes_the_three_forms() {
    let t = share_text("Scrivetemi a mario.rossi@studenti.unito.it. Matricola 123456, tel +39 333 1234567 o 011 6707 123: perché il picco ha una coda?").unwrap();
    assert!(
        !t.contains('@') && !t.contains("123456") && !t.contains("1234567") && !t.contains("6707"),
        "{t}"
    );
    assert!(t.contains("perché il picco ha una coda?"));
    assert_eq!(
        share_text("La retta ha R2 0,98 a m/z 266, RT 4,2 min: perché?").unwrap(),
        "La retta ha R2 0,98 a m/z 266, RT 4,2 min: perché?"
    );
    assert_eq!(share_text("011-6707123 ?"), None);
    assert_eq!(share_text(&"a".repeat(601)), None);
}

#[test]
fn counters_hold_no_text() {
    let mut s = Stat::new("LAB3");
    s.add(
        "2026-10-21",
        &Record::Answer {
            passi: false,
            intent: Intent::Concetto,
            blocks: vec!["09-quadrupolo#stabilita.1".into()],
            uncovered: false,
            tokens_in: 5700,
            tokens_out: 900,
            cost_micros: 14_700,
        },
    );
    s.add(
        "2026-10-21",
        &Record::Vote {
            negative: true,
            reason: Some("wrong".into()),
            cited: vec!["09-quadrupolo#stabilita.1".into()],
        },
    );
    s.add(
        "2026-10-21",
        &Record::Vote {
            negative: true,
            reason: Some("mario.rossi@x.it".into()),
            cited: vec![],
        },
    );
    s.add(
        "2026-10-22",
        &Record::Vote {
            negative: false,
            reason: None,
            cited: vec![],
        },
    );
    let j = s.to_json();
    assert_eq!(j["schema"], "tutorstat/1");
    let d = &j["days"]["2026-10-21"];
    assert_eq!(d["questions"], 1);
    assert_eq!(d["votes_down"]["wrong"], 1);
    // a free-text reason is counted as "other", never kept
    assert_eq!(d["votes_down"]["other"], 1);
    assert!(!j.to_string().contains('@'));
    assert_eq!(j["days"]["2026-10-22"]["votes_up"], 1);
    same_as_file("tutorstat-esempio.json", &j.to_string());
}
