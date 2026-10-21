//! The blocks of the real Teoria: determinism, size, unique ids, anchors that exist, and no text of the hidden answers.
use mzlab_tutor::blocchi::{build, shingles, to_text};
use std::{collections::HashSet, fs, path::PathBuf};

fn teoria_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../mzlab/web/teoria")
}

fn pages() -> Vec<(String, String)> {
    let mut v = Vec::new();
    for e in fs::read_dir(teoria_dir()).unwrap() {
        let p = e.unwrap().path();
        if p.extension().is_some_and(|x| x == "html") {
            v.push((
                p.file_name().unwrap().to_str().unwrap().to_string(),
                fs::read_to_string(&p).unwrap(),
            ));
        }
    }
    v
}

fn page(name: &str, html: &str) -> serde_json::Value {
    build(&[(name.to_string(), html.to_string())]).unwrap()
}

#[test]
fn same_bytes_twice() {
    let mut reversed = pages();
    let a = to_text(&build(&pages()).unwrap());
    reversed.reverse();
    assert_eq!(a, to_text(&build(&reversed).unwrap()));
}

#[test]
fn ids_unique_and_blocks_short() {
    let v = build(&pages()).unwrap();
    let mut seen = HashSet::new();
    for b in v["blocks"].as_array().unwrap() {
        let id = b["id"].as_str().unwrap();
        assert!(seen.insert(id.to_string()), "duplicate id {id}");
        assert!(
            b["text"].as_str().unwrap().chars().count() <= 1200,
            "{id} is too long"
        );
        assert!(id.starts_with(&format!(
            "{}#{}.",
            b["file"].as_str().unwrap(),
            b["anchor"].as_str().unwrap()
        )));
    }
    assert_eq!(v["n"].as_u64().unwrap() as usize, seen.len());
}

#[test]
fn anchors_exist_in_the_page() {
    let pages = pages();
    let v = build(&pages).unwrap();
    for b in v["blocks"].as_array().unwrap() {
        let (file, anchor) = (b["file"].as_str().unwrap(), b["anchor"].as_str().unwrap());
        if anchor.is_empty() {
            continue;
        }
        let html = &pages
            .iter()
            .find(|p| p.0 == format!("{file}.html"))
            .unwrap()
            .1;
        // headings get their id at run time (teoria.js): either written in the page or the slug of the heading text
        let written = html.contains(&format!("id=\"{anchor}\""));
        let heading = html.split("<h").skip(1).any(|h| {
            let inner = h[h.find('>').unwrap_or(0) + 1..]
                .split("</h")
                .next()
                .unwrap_or("");
            mzlab_libro::heading_anchor(inner).is_ok_and(|a| a == anchor)
        });
        assert!(written || heading, "{file}#{anchor} is not in the page");
    }
}

#[test]
fn no_hidden_answer_in_the_texts() {
    let pages = pages();
    let v = build(&pages).unwrap();
    let all = mzlab_libro::norm(
        &v["blocks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| b["text"].as_str().unwrap())
            .collect::<Vec<_>>()
            .join(" "),
    );
    let mut checked = 0;
    for (_, html) in &pages {
        for det in html.split("<details class=\"q\">").skip(1) {
            let body = det.split("</details>").next().unwrap();
            let answer = body.split("</summary>").nth(1).unwrap_or("");
            let plain = mzlab_libro::norm(
                &answer
                    .split('<')
                    .map(|s| s.split_once('>').map_or(s, |x| x.1))
                    .collect::<String>(),
            );
            let probe: String = plain.chars().take(60).collect();
            if probe.chars().count() == 60 {
                assert!(!all.contains(&probe), "answer text leaked: {probe}");
                checked += 1;
            }
        }
    }
    assert!(checked > 40, "only {checked} answers checked");
}

#[test]
fn exercises_carry_only_the_fingerprint() {
    let v = page("99-x.html", "<main><h2>Verifica</h2><details class=\"q\"><summary>Quanto fa 2+2?</summary><p>La risposta è quattro, come ben sapete tutti quanti voi.</p></details></main>");
    let b = &v["blocks"][0];
    assert_eq!(b["kind"], "esercizio");
    assert_eq!(b["text"], "Quanto fa 2+2?");
    assert_eq!(b["hidden_sha"].as_str().unwrap().len(), 12);
    assert_eq!(
        b["hidden_shingles"].as_array().unwrap().len(),
        shingles("La risposta è quattro, come ben sapete tutti quanti voi.").len()
    );
    assert!(!v.to_string().contains("quattro"));
}

#[test]
fn redirects_are_skipped_and_glossary_is_one_block_per_entry() {
    let v = build(&[
        ("a.html".into(), "<meta http-equiv=\"refresh\" content=\"0;url=b.html\"><main><p>Rimando</p></main>".into()),
        ("b.html".into(), "<main><dl><dt id=\"g-esi\">ESI</dt><dd>Elettrospray: <b>ionizza</b> un liquido.</dd><dt id=\"g-ms\">MS</dt><dd>Massa.</dd></dl></main>".into()),
    ])
    .unwrap();
    let b = v["blocks"].as_array().unwrap();
    assert_eq!(b.len(), 2);
    assert_eq!(b[0]["text"], "ESI: Elettrospray: ionizza un liquido.");
    assert_eq!(
        (b[0]["kind"].as_str(), b[0]["anchor"].as_str()),
        (Some("glossario"), Some("g-esi"))
    );
}

#[test]
fn figure_description_joins_the_caption_and_long_paragraphs_are_cut() {
    let long = "Una frase abbastanza lunga da riempire lo spazio. ".repeat(40);
    let html = format!("<main><h2>Figure</h2><figure><svg><text>no</text></svg><figcaption>Figura 1 Didascalia.</figcaption><details class=\"figdesc\"><summary>Descrizione della figura</summary><p>Due riquadri.</p></details></figure><p>{long}</p></main>");
    let v = page("98-y.html", &html);
    let b = v["blocks"].as_array().unwrap();
    assert_eq!(b[0]["kind"], "figura");
    assert_eq!(b[0]["text"], "Figura 1 Didascalia. Due riquadri.");
    assert!(
        b.len() >= 3
            && b.iter()
                .all(|x| x["text"].as_str().unwrap().chars().count() <= 1200)
    );
    assert!(!v.to_string().contains("Descrizione della figura"));
}
