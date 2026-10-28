//! The dissociation methods that have a PSI-MS term: (name as the file formats spell it, accession, term name). The mzML writer (`mzmlwrite::attivazione_da_nome`) and
//! the scan table reader (`scantable::precursor_element`) share this one table, so that what the writer can write the reader names back.

/// In the order the reader looks them up: the first whose accession is in the activation wins.
pub const ATTIVAZIONI: [(&str, &str, &str); 9] = [
    (
        "HCD",
        "MS:1000422",
        "beam-type collision-induced dissociation",
    ),
    ("CID", "MS:1000133", "collision-induced dissociation"),
    ("ETD", "MS:1000598", "electron transfer dissociation"),
    ("ECD", "MS:1000250", "electron capture dissociation"),
    ("PQD", "MS:1000599", "pulsed q dissociation"),
    ("IRMPD", "MS:1000262", "infrared multiphoton dissociation"),
    ("UVPD", "MS:1003246", "ultraviolet photodissociation"),
    ("MPID", "MS:1000435", "photodissociation"),
    ("SID", "MS:1000136", "surface-induced dissociation"),
];

/// The (accession, term) of the method named `nome` (any case, spaces around ignored).
pub fn da_nome(nome: &str) -> Option<(&'static str, &'static str)> {
    let n = nome.trim();
    ATTIVAZIONI
        .iter()
        .find(|a| a.0.eq_ignore_ascii_case(n))
        .map(|a| (a.1, a.2))
}

/// The name of the first method of the table that the element (the text of a `<precursor>`) has as `accession="..."`.
pub fn nome_in(e: &str) -> Option<&'static str> {
    ATTIVAZIONI
        .iter()
        .find(|a| ha_accession(e, a.1))
        .map(|a| a.0)
}

/// Whether `e` has `accession="<acc>"` (no allocation: the terms are looked up for every precursor).
fn ha_accession(e: &str, acc: &str) -> bool {
    let mut from = 0;
    while let Some(i) = e[from..].find(acc) {
        let at = from + i;
        if e[..at].ends_with("accession=\"") && e[at + acc.len()..].starts_with('"') {
            return true;
        }
        from = at + 1;
    }
    false
}
