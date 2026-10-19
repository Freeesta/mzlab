// Ported from Attributio vocab.py.
//! The English form of the words that are data in the snapshots. Files written before 2026-09-26 carry Italian words; `canon` reads them (never written).

pub const TRUE: &str = "true";
pub const DOUBTFUL: &str = "doubtful";
pub const FALSE: &str = "false";
/// The list entries a structural screening can use.
pub const AMENABLE: [&str; 2] = [TRUE, DOUBTFUL];

const ALIASES: &[(&str, &str)] = &[
    ("vero", TRUE),
    ("dubbio", DOUBTFUL),
    ("falso", FALSE),
    ("si", "yes"),
    ("sì", "yes"),
    ("Livello 2a", "Level 2a"),
    ("Livello 3", "Level 3"),
    ("Livello 3 con riserva", "Level 3 with reservation"),
    ("respinto", "rejected"),
    ("non confermato", "unconfirmed"),
    ("struttura della fonte", "source structure"),
    ("CID della fonte", "source CID"),
    ("nome", "name"),
    ("registro (CAS)", "registry (CAS)"),
    ("voce", "entry"),
    ("registro", "registry"),
    (
        "CAS non nel registro chimico ne' in NORMAN SusDat",
        "CAS in neither the chemical registry nor NORMAN SusDat",
    ),
    (
        "senza CAS (PubChem per nome)",
        "without CAS (PubChem by name)",
    ),
    ("costruita", "built"),
    ("non costruita", "not built"),
    ("avviso", "warning"),
    ("copertura", "coverage"),
    ("massa", "mass"),
    ("nome_inchikey", "name_inchikey"),
    ("categorie", "categories"),
    ("tossicita", "toxicity"),
    ("PubChem name (registro)", "PubChem name (registry)"),
];

/// The English form of a vocabulary value; anything else is returned unchanged (`vocab.canon`, whole values and values joined by « ; » or « | »).
pub fn canon(value: &str) -> String {
    let s = value.trim();
    if let Some((_, en)) = ALIASES.iter().find(|(it, _)| *it == s) {
        return en.to_string();
    }
    // Verdicts followed by more text: the failed rules, or a chemist's variant such as «Livello 3 (...)».
    for (it, en) in [
        ("non confermato (", "unconfirmed ("),
        ("Livello ", "Level "),
    ] {
        if let Some(rest) = s.strip_prefix(it) {
            return format!("{en}{rest}");
        }
    }
    for sep in [" ; ", " | "] {
        if s.contains(sep) {
            return s.split(sep).map(canon).collect::<Vec<_>>().join(sep);
        }
    }
    value.to_string()
}
