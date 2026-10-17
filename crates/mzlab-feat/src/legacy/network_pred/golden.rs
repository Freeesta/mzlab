//! `predecessors` against the golden file of the reference (`tests/golden/finder_funzioni.json`).

use super::*;

fn rules() -> Rules {
    Rules::parse(include_bytes!("../../../../../TP_Mine/data/regole_tp.json")).unwrap()
}

/// The choices of `predecessors` on the cases of `tests/golden/finder_funzioni.json` (names differ: the table of the reference is its own).
#[test]
fn the_predecessors_match_the_python_reference() {
    let g: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../tests/golden/finder_funzioni.json"
    ))
    .unwrap();
    let r = rules();
    let parse = |f: &str| -> Counts {
        let mut out = Counts::new();
        let (mut el, mut num) = (String::new(), String::new());
        let mut flush = |el: &mut String, num: &mut String| {
            if !el.is_empty() {
                out.insert(std::mem::take(el), num.parse().unwrap_or(1));
            }
            num.clear();
        };
        for ch in f.chars() {
            if ch.is_ascii_digit() {
                num.push(ch);
            } else {
                if ch.is_ascii_uppercase() {
                    flush(&mut el, &mut num);
                }
                el.push(ch);
            }
        }
        flush(&mut el, &mut num);
        out
    };
    for case in g["predecessors"].as_array().unwrap() {
        let mut forms = vec![parse("C8H11N4O2")];
        forms.extend(
            case["forms"]
                .as_array()
                .unwrap()
                .iter()
                .map(|f| parse(f.as_str().unwrap())),
        );
        let mut tmax = vec![Some(0.0)];
        tmax.extend(case["tmax"].as_array().unwrap().iter().map(|t| t.as_f64()));
        let cos_in: Vec<(usize, usize, f64, u64)> = case["cos"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| {
                (
                    e[0].as_u64().unwrap() as usize,
                    e[1].as_u64().unwrap() as usize,
                    e[2].as_f64().unwrap(),
                    e[3].as_u64().unwrap(),
                )
            })
            .collect();
        let cos = |j: usize, i: usize| {
            cos_in
                .iter()
                .find(|e| e.0 + 1 == j && e.1 + 1 == i)
                .filter(|e| e.2 >= COS_RELATED && e.3 as usize >= MIN_MATCHED)
                .map(|e| e.2)
        };
        let got = predecessors(&forms, &tmax, &r, &cos);
        for want in case["pred"].as_array().unwrap() {
            let to = want["to"].as_u64().unwrap() as usize;
            let from = want["from"].as_u64().map(|v| v as usize);
            let have = got.iter().find(|p| p.to == to).map(|p| p.from);
            assert_eq!(have, from, "case {case}, node {to}");
        }
    }
}
