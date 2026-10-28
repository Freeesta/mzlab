//! Words of a file name that suggest a blank (`wash`, `MQ`, `milliq`...), read from `data/nomi-bianco.json`; used by `nomi.rs` for the role Auto.
//! A name with a time or an extra series token (`MQ_PMS_T00`) is a sample: the matrix MilliQ is not a blank.

use std::sync::OnceLock;

struct Table {
    words: Vec<String>,
    neutral: Vec<String>,
}

fn table() -> &'static Table {
    static T: OnceLock<Table> = OnceLock::new();
    T.get_or_init(|| {
        let v = crate::json::parse(include_bytes!("../../data/nomi-bianco.json"))
            .expect("nomi-bianco.json is valid");
        let list = |k: &str| -> Vec<String> {
            v.get(k)
                .map(|a| {
                    a.arr()
                        .iter()
                        .filter_map(|x| x.str().map(String::from))
                        .collect()
                })
                .unwrap_or_default()
        };
        Table {
            words: list("words"),
            neutral: list("neutral"),
        }
    })
}

/// A token is a word or a word followed by digits only (`wash1`).
fn is_word(tok: &str, word: &str) -> bool {
    tok.starts_with(word) && tok[word.len()..].chars().all(|c| c.is_ascii_digit())
}

/// Whether the lowercase name (without folders and extension) reads as a blank: a blank word is there and every other letter token is neutral
/// (digits, polarity, the `q` of `milli-q`); a token with a time (`t00`, `5min`) is not neutral.
pub fn is_blank_name(stem_low: &str) -> bool {
    let t = table();
    let toks: Vec<&str> = stem_low
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| !w.is_empty())
        .collect();
    let hit = |w: &&str| t.words.iter().any(|x| is_word(w, x));
    toks.iter().any(hit)
        && toks.iter().all(|w| {
            hit(w) || w.chars().all(|c| c.is_ascii_digit()) || t.neutral.iter().any(|n| n == w)
        })
}
