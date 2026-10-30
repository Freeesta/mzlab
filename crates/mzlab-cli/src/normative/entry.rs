//! What a reader hands to `resolve`: one entry of a legal text, as printed, with its kind. Also the CAS helpers shared by the readers.

const GROUP_WORDS: [&str; 15] = [
    "sum of",
    "total ",
    "and related",
    "related substances",
    "related compounds",
    "its salts",
    "their salts",
    "group",
    "compounds",
    "isomers",
    "derivatives",
    "individual ",
    "pesticides",
    "pharmaceutical",
    "metabolites",
];
const UVCB_WORDS: [&str; 9] = [
    "uvcb",
    "technical",
    "reaction mass",
    "reaction products",
    "mixture",
    "alkanes, c",
    "chlorinated paraffins",
    "aroclor",
    "extracts",
];
const INORGANIC: [&str; 40] = [
    "lead",
    "cadmium",
    "mercury",
    "chromium",
    "chromium vi",
    "nickel",
    "zinc",
    "copper",
    "arsenic",
    "antimony",
    "beryllium",
    "selenium",
    "silver",
    "thallium",
    "barium",
    "boron",
    "cobalt",
    "tin",
    "uranium",
    "manganese",
    "molybdenum",
    "vanadium",
    "aluminium",
    "aluminum",
    "iron",
    "lithium",
    "asbestos",
    "cyanide",
    "cyanides",
    "total cyanide",
    "nitrate",
    "nitrite",
    "nitrates",
    "chlorate",
    "chlorite",
    "perchlorate",
    "fluoride",
    "bromate",
    "ammonium",
    "sulphate",
];

impl Kind {
    pub fn word(self) -> &'static str {
        match self {
            Kind::Structure => "screenable",
            Kind::Group => "group",
            Kind::Uvcb => "uvcb",
            Kind::Inorganic => "inorganic",
        }
    }
}

impl Entry {
    pub fn new(entry_ref: &str, name: &str, cas: &str) -> Entry {
        let name = name.split_whitespace().collect::<Vec<_>>().join(" ");
        let open_ended = cas.to_lowercase().contains("others");
        let cas = cas_tokens(cas).join(" ; ");
        let mut kind = classify(&name, &cas);
        if open_ended && kind == Kind::Structure {
            kind = Kind::Group;
        }
        Entry {
            entry_ref: entry_ref.trim().into(),
            kind,
            name,
            cas,
            ec: String::new(),
            dtxsid: String::new(),
            attr: String::new(),
            group_entry: String::new(),
            law_key: String::new(),
        }
    }
}

/// What an entry names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// One compound that can have a structure.
    Structure,
    /// An open group or a definition by structure ("sum of ...", "and related substances").
    Group,
    /// A mixture or a substance of unknown or variable composition.
    Uvcb,
    /// An element, a metal or an inorganic ion.
    Inorganic,
}

#[derive(Clone, Debug)]
pub struct Entry {
    /// Annex, table and entry number as the text numbers them.
    pub entry_ref: String,
    pub name: String,
    /// CAS numbers as printed (several separated by " ; ").
    pub cas: String,
    pub ec: String,
    pub dtxsid: String,
    /// The value that makes the entry matter (EQS, MCL, status), as printed.
    pub attr: String,
    /// The text of the enumerated group this entry is a member of.
    pub group_entry: String,
    /// A full InChIKey when the text itself gives the structure (rare).
    pub law_key: String,
    pub kind: Kind,
}

/// Whether `s` has the check digit of a CAS number (only used to tell a curator that a number in the text may be a typo).
pub fn cas_check_ok(s: &str) -> bool {
    if !is_cas(s) {
        return false;
    }
    let p: Vec<&str> = s.split('-').collect();
    let digits: Vec<u32> = format!("{}{}", p[0], p[1])
        .bytes()
        .map(|b| u32::from(b - b'0'))
        .collect();
    let sum: u32 = digits
        .iter()
        .rev()
        .enumerate()
        .map(|(i, d)| d * (i as u32 + 1))
        .sum();
    sum % 10 == u32::from(p[2].as_bytes()[0] - b'0')
}

/// Whether `s` has the shape of a CAS number: 2 to 7 digits, 2 digits, one digit.
pub fn is_cas(s: &str) -> bool {
    let p: Vec<&str> = s.split('-').collect();
    p.len() == 3
        && (2..=7).contains(&p[0].len())
        && p[1].len() == 2
        && p[2].len() == 1
        && p.iter().all(|x| x.bytes().all(|b| b.is_ascii_digit()))
}

/// The CAS numbers in a text, in order, without repeats (by shape; the check digit is not tested, a typo in the law must still reach the curator).
pub fn cas_tokens(text: &str) -> Vec<String> {
    let t: String = text
        .chars()
        .map(|c| {
            if matches!(c, '\u{2010}'..='\u{2015}' | '\u{2212}') {
                '-'
            } else {
                c
            }
        })
        .collect();
    let b = t.as_bytes();
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if !b[i].is_ascii_digit()
            || (i > 0 && (b[i - 1].is_ascii_alphanumeric() || b[i - 1] == b'-'))
        {
            i += 1;
            continue;
        }
        let mut j = i;
        while j < b.len() && (b[j].is_ascii_digit() || b[j] == b'-') {
            j += 1;
        }
        let tok = &t[i..j];
        if is_cas(tok) && !out.iter().any(|o| o == tok) {
            out.push(tok.to_string());
        }
        i = j;
    }
    out
}

/// The kind an entry has by its name, by rule: the same text always gets the same kind. A name that is only an element or an ion is inorganic.
pub fn classify(name: &str, _cas: &str) -> Kind {
    let n = name.to_lowercase();
    let n = n
        .trim_end_matches(" and compounds")
        .trim_end_matches(" and its compounds")
        .trim();
    if INORGANIC.contains(&n) {
        return Kind::Inorganic;
    }
    if UVCB_WORDS.iter().any(|w| n.contains(w)) {
        return Kind::Uvcb;
    }
    if GROUP_WORDS.iter().any(|w| n.contains(w)) {
        return Kind::Group;
    }
    Kind::Structure
}
