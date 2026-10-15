// Ported from Attributio names.py.
//! The canonical common name of a compound identity. The same compound turns up under different names (systematic names, synonyms, brand names, library
//! codes), so matching on the raw name loses real pairs; the formula alone would merge isomers. `canonical_name` therefore (1) cleans dirty library strings,
//! (2) unifies VERIFIED synonyms only (never isomers), (3) groups the polyether series by formula and (4) otherwise returns the cleaned name.

use super::pattern::Pattern;
use std::sync::OnceLock;

/// Verified same-compound pairs; the key is the raw name, trimmed and lowercase.
const SYNONYMS: &[(&str, &str)] = &[
    ("atcardil", "Atenolol (beta-blocker)"),
    ("aerodiol", "Estradiol (hormone)"),
    ("8phic8spc", "Sulfophenyl carboxylate (LAS metab.)"),
    ("10phic10spc", "Sulfophenyl carboxylate (LAS metab.)"),
    ("zlchem 800", "4-Methyl-2-mercaptobenzimidazole"),
    ("z1579222596", "3-(3,5-di-tert-butyl-4-hydroxyphenyl)prop-2-enoic acid (BHT metab.)"),
    (
        "3-(3,5-di-tert-butyl-4-hydroxyphenyl)prop-2-enoic acid",
        "3-(3,5-di-tert-butyl-4-hydroxyphenyl)prop-2-enoic acid (BHT metab.)",
    ),
    ("p-xanthine", "Paraxanthine (caffeine metab.)"),
    ("alpha-la", "alpha-Linolenic acid (ALA)"),
    ("bepantol / dexpanthenol (skincare)", "Dexpanthenol / Panthenol (provit. B5)"),
    ("panthenol (provit. b5)", "Dexpanthenol / Panthenol (provit. B5)"),
    ("hexaethylene glycol", "Hexaethylene glycol (PEG6)"),
    (
        "diethylene glycol adipate cyclic ester (polyester oligomer)",
        "Diethylene glycol adipate cyclic ester (polyester oligomer)",
    ),
    ("peg (hexaethylene glycol)", "Hexaethylene glycol (PEG6)"),
    ("terbuthylazine (incl. tp)", "Terbuthylazine (incl. TP)"),
    (
        "massbank: terbuthylazine-tp cscd648241 (lm6)|3-methyl-6-[(2-methyl-2-propanyl)amino]-1,3,5-triazine-2,4(1h,3h)-dione",
        "Terbuthylazine (incl. TP)",
    ),
    ("linear alkylbenzene sulfonate (c10-las)", "Linear alkylbenzene sulfonate (C10-LAS)"),
    ("c10-las", "Linear alkylbenzene sulfonate (C10-LAS)"),
    ("cholalin", "Cholic acid (bile acid)"),
    ("t-alpha-mc", "Tauro-alpha-muricholic acid (bile acid)"),
    ("taurolite", "Tauroursodeoxycholic acid (bile acid, TUDCA)"),
    ("tudcabil", "Tauroursodeoxycholic acid (bile acid, TUDCA)"),
    ("prunetol", "Genistein (isoflavone)"),
    ("sialate", "N-Acetylneuraminic acid (sialic acid)"),
];

fn synonym(key: &str) -> Option<&'static str> {
    SYNONYMS.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

fn peg_hint() -> &'static Pattern {
    static P: OnceLock<Pattern> = OnceLock::new();
    P.get_or_init(|| {
        Pattern::new(
            r"polyethylene glycol|\bpeg\b|hexaoxa|pentaoxa|tetraoxa|trioxa|nonaoxa|oxanonadec|ethoxyethoxy|methoxyethane|nonaoxacyclo|-oxa[a-z]*cyclo|ethylene glycol|peg vinyl ether|peg diol|peg monomethyl",
        )
    })
}

fn unresolved() -> &'static Pattern {
    static P: OnceLock<Pattern> = OnceLock::new();
    P.get_or_init(|| Pattern::new(r"^(acmc-|ncgc\d|oprea\d|bmse\d|stk\d|z\d{6,}|cid[\s:]|mls\d)"))
}

/// Formulas are plain ASCII everywhere (`C10H22O6`); an absent formula is empty. Kept for the exports that call it by this name.
pub fn subscript_formula(text: Option<&str>) -> String {
    text.unwrap_or("").to_string()
}

/// `\s*\((9ci|lm\d+|tentative|sample)\)\s*` replaced by one space, case-insensitive.
fn drop_qualifiers(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if let Some(end) = qualifier_end(&chars, i) {
            while out.ends_with(char::is_whitespace) {
                out.pop();
            }
            out.push(' ');
            i = end;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    out
}

/// The index after a qualifier (and the whitespace around it) that starts at `i` (at its whitespace or its parenthesis).
fn qualifier_end(c: &[char], i: usize) -> Option<usize> {
    let mut j = i;
    while c.get(j).is_some_and(|x| x.is_whitespace()) {
        j += 1;
    }
    if c.get(j) != Some(&'(') {
        return None;
    }
    let close = c[j + 1..].iter().position(|&x| x == ')')? + j + 1;
    let word: String = c[j + 1..close].iter().collect::<String>().to_lowercase();
    let digits = word
        .strip_prefix("lm")
        .is_some_and(|d| !d.is_empty() && d.chars().all(|x| x.is_ascii_digit()));
    if !(digits || ["9ci", "tentative", "sample"].contains(&word.as_str())) {
        return None;
    }
    let mut end = close + 1;
    while c.get(end).is_some_and(|x| x.is_whitespace()) {
        end += 1;
    }
    Some(end)
}

/// A raw library string cleaned: the part before `|`, without the `Massbank:` prefix and the `(9CI)`, `(LMn)`, `(tentative)`, `(sample)` qualifiers.
pub fn clean_raw(name: &str) -> String {
    let mut s = name.trim();
    if let Some((a, _)) = s.split_once('|') {
        s = a;
    }
    let t = s.trim_start();
    let s = if t.to_lowercase().starts_with("massbank:") {
        t["massbank:".len()..].trim_start()
    } else {
        s
    };
    let s = drop_qualifiers(s);
    let joined = s.split_whitespace().collect::<Vec<_>>().join(" ");
    joined.trim_matches(|c| c == ' ' || c == '-').to_string()
}

/// `identity isomer, RT x.x min` split into the identity and the RT text.
pub fn split_rt_isomer(raw: &str) -> Option<(&str, &str)> {
    let body = raw.strip_suffix(" min")?;
    let (head, rt) = body.rsplit_once(" isomer, RT ")?;
    let ok = !rt.is_empty()
        && rt.chars().all(|c| c.is_ascii_digit() || c == '.')
        && rt.split('.').count() <= 2
        && !rt.starts_with('.')
        && !rt.ends_with('.');
    (ok && !head.is_empty() && !head.ends_with(char::is_whitespace)).then_some((head, rt))
}

/// The first `[Formula]` of a text (`\[([A-Za-z0-9]+)\]`).
fn bracketed_formula(raw: &str) -> Option<&str> {
    let mut from = 0;
    while let Some(o) = raw[from..].find('[') {
        let start = from + o + 1;
        let body = &raw[start..];
        let n = body
            .find(|c: char| !c.is_ascii_alphanumeric())
            .unwrap_or(body.len());
        if n > 0 && body[n..].starts_with(']') {
            return Some(&body[..n]);
        }
        from = start;
    }
    None
}

/// The canonical common name of `identity`; `formula` is used only to group the polyether series. The suffix ` isomer, RT x.x min` of the retention-time rule
/// is kept: an isomer never merges back into the compound it was split from.
pub fn canonical_name(identity: &str, formula: Option<&str>) -> String {
    let raw = identity.trim();
    if let Some((head, rt)) = split_rt_isomer(raw) {
        return format!("{} isomer, RT {rt} min", canonical_name(head, formula));
    }
    if let Some(v) = synonym(&raw.to_lowercase()) {
        return v.to_string();
    }
    let cleaned = clean_raw(raw);
    let ckey = cleaned.to_lowercase();
    if let Some(v) = synonym(&ckey) {
        return v.to_string();
    }
    if peg_hint().search(&ckey) {
        let f = formula
            .filter(|f| !f.is_empty())
            .or_else(|| bracketed_formula(raw));
        return match f {
            Some(f) => format!("PEG/polyether [{}]", subscript_formula(Some(f))),
            None => "PEG/polyether".to_string(),
        };
    }
    if unresolved().search(&ckey) {
        return format!("(unresolved library code: {cleaned})");
    }
    cleaned
}
