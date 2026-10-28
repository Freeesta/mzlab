//! Tests of the CSV byte scanner: equality with the first char-by-char parser on every short text and on random ones, and its timing.

use super::csv::*;
use super::csv_campi::{records, Cells};
use crate::error::Result;


/// The first parser, char by char, kept as the reference the byte scanner is compared with.
/// (was) quoted fields keep their separators and newlines (`""` is a quote); a field is trimmed unless quoted.
fn records_ref(text: &str, sep: char) -> Result<Vec<Vec<(String, bool)>>> {
    let mut out: Vec<Vec<(String, bool)>> = Vec::new();
    let (mut row, mut cur, mut quoted, mut inq) =
        (Vec::<(String, bool)>::new(), String::new(), false, false);
    let mut it = text.trim_start_matches('\u{feff}').chars().peekable();
    let end = |row: &mut Vec<(String, bool)>, cur: &mut String, quoted: &mut bool| {
        let v = if *quoted {
            std::mem::take(cur)
        } else {
            cur.trim().to_string()
        };
        cur.clear();
        row.push((v, std::mem::take(quoted)));
    };
    while let Some(c) = it.next() {
        if inq {
            match (c, it.peek()) {
                ('"', Some('"')) => {
                    cur.push('"');
                    it.next();
                }
                ('"', _) => inq = false,
                _ => cur.push(c),
            }
        } else if c == '"' && cur.trim().is_empty() {
            cur.clear();
            inq = true;
            quoted = true;
        } else if c == sep {
            end(&mut row, &mut cur, &mut quoted);
        } else if c == '\n' || c == '\r' {
            if c == '\r' && it.peek() == Some(&'\n') {
                it.next();
            }
            end(&mut row, &mut cur, &mut quoted);
            let blank = row.len() == 1 && row[0].0.is_empty() && !row[0].1;
            let r = std::mem::take(&mut row);
            if !blank {
                out.push(r);
            }
        } else {
            cur.push(c);
        }
    }
    if inq {
        return Err(bad(out.len() + 1, ""));
    }
    if !cur.is_empty() || !row.is_empty() {
        end(&mut row, &mut cur, &mut quoted);
        out.push(row);
    }
    Ok(out)
}

fn owned(r: Result<Vec<Cells<'_>>>) -> Result<Vec<Vec<(String, bool)>>> {
    r.map(|rows| {
        rows.into_iter()
            .map(|r| r.into_iter().map(|(v, q)| (v.into_owned(), q)).collect())
            .collect()
    })
}

fn same(text: &str, sep: char) {
    assert_eq!(
        owned(records(text, sep)),
        records_ref(text, sep),
        "{text:?} sep {sep:?}"
    );
}

/// Every text up to 6 symbols of an alphabet made of the awkward ones, for the three separators.
#[test]
fn byte_scanner_equals_char_parser_on_every_short_text() {
    let alpha = ['a', ' ', '"', ',', ';', '\n', '\r', 'é'];
    for sep in [',', ';', '\t'] {
        let mut texts = vec![String::new()];
        let mut level = vec![String::new()];
        for _ in 0..6 {
            level = level
                .iter()
                .flat_map(|t| alpha.iter().map(move |c| format!("{t}{c}")))
                .collect();
            texts.extend(level.iter().cloned());
        }
        for t in &texts {
            same(t, sep);
        }
    }
}

#[test]
fn byte_scanner_equals_char_parser_on_random_texts() {
    let alpha = [
        'a', 'b', ' ', ' ', '"', '"', ',', ';', '\t', '\n', '\r', 'é', '€', '\u{a0}',
        '\u{feff}',
    ];
    let mut s = 0x9e3779b97f4a7c15u64;
    for _ in 0..200_000 {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let len = (s >> 60) as usize + (s >> 20 & 0x1f) as usize;
        let mut t = String::new();
        let mut x = s;
        for _ in 0..len {
            x = x
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            t.push(alpha[(x >> 33) as usize % alpha.len()]);
        }
        same(&t, [',', ';', '\t', 'é'][(s >> 7) as usize % 4]);
    }
}

#[test]
fn records_of_the_awkward_cases() {
    let cells = |t: &str| owned(records(t, ',')).unwrap();
    let c = |v: &str, q: bool| (v.to_string(), q);
    assert_eq!(
        cells("a,\"x,y\",c\n"),
        vec![vec![c("a", false), c("x,y", true), c("c", false)]]
    );
    assert_eq!(
        cells("\"l1\nl2\",z"),
        vec![vec![c("l1\nl2", true), c("z", false)]]
    );
    assert_eq!(
        cells("\"say \"\"hi\"\"\""),
        vec![vec![c("say \"hi\"", true)]]
    );
    assert_eq!(
        cells("ab\"c,d"),
        vec![vec![c("ab\"c", false), c("d", false)]]
    );
    assert_eq!(
        cells("\"abc\" ,d"),
        vec![vec![c("abc ", true), c("d", false)]]
    );
    assert_eq!(
        cells("a\rb\r\nc\n\n\nd"),
        vec![
            vec![c("a", false)],
            vec![c("b", false)],
            vec![c("c", false)],
            vec![c("d", false)]
        ]
    );
    assert_eq!(
        cells("\u{feff}a,é€\n"),
        vec![vec![c("a", false), c("é€", false)]]
    );
    assert_eq!(
        cells("  x  ,\"  y  \""),
        vec![vec![c("x", false), c("  y  ", true)]]
    );
    assert!(records("\"open", ',').is_err());
}

/// 10^5 rows of 12 columns: the two parsers, best of 5.
#[test]
#[ignore = "timing"]
fn timing_of_the_parsers() {
    let mut t = String::from("file,sha256,campione_id,polarita,gruppo,tipo,tempo,unita_tempo,replica,concentrazione,unita,note\n");
    for i in 0..100_000 {
        t += &format!(
            "run_{i:06}.mzML,{:064x},S{i},pos,g{},sample,{}.5,min,{},{}.25,ppm,\"note, {i}\"\n",
            i * 7919,
            i % 9,
            i % 40,
            i % 3 + 1,
            i % 50
        );
    }
    let best = |f: &dyn Fn() -> usize| {
        (0..5)
            .map(|_| {
                let t0 = std::time::Instant::now();
                let n = f();
                assert_eq!(n, 100_001);
                t0.elapsed().as_secs_f64() * 1e3
            })
            .fold(f64::MAX, f64::min)
    };
    let old = best(&|| records_ref(&t, ',').unwrap().len());
    let new = best(&|| records(&t, ',').unwrap().len());
    let all = best(&|| leggi(&t, None, &[]).unwrap().campioni.len() + 1);
    eprintln!("records old {old:.2} ms · new {new:.2} ms · leggi (whole) {all:.2} ms");
}
