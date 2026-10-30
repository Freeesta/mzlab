//! The user's own CSV of the ECHA Candidate List (`svhc`) or of OEHHA's Proposition 65 list (`ca_prop65`) as entries: the same code in the `.wasm`, the app
//! and the CLI reads it.

use super::csvrd::{at, col, guess_sep, Records};
use super::entry::{cas_tokens, Entry, Kind};
use super::Error;

/// Appends the entries of `text` to `out`; `svhc` picks the label of the entry numbers.
pub fn entries(id: &str, svhc: bool, text: &str, out: &mut Vec<Entry>) -> Result<(), Error> {
    let sep = guess_sep(text.lines().next().unwrap_or(""));
    let mut it = Records::new(text.as_bytes(), sep);
    let head = it.next().unwrap_or_default();
    let name_c = col(
        &head,
        &["Substance name", "Chemical", "Chemical name", "Name"],
    );
    let cas_c = col(
        &head,
        &["CAS no.", "CAS No.", "CAS Number", "CAS", "CAS RN"],
    );
    let ec_c = col(&head, &["EC / List no.", "EC No.", "EC number"]);
    let attr_c = col(
        &head,
        &["Reason for inclusion", "Type of Toxicity", "Toxicity"],
    );
    let date_c = col(&head, &["Date of inclusion", "Date Listed", "Date"]);
    if name_c.is_none() || cas_c.is_none() {
        return Err(Error::parse(id, "the file has no name and CAS columns"));
    }
    for rec in it {
        let name = at(&rec, name_c);
        if name.is_empty() {
            continue;
        }
        let cas = at(&rec, cas_c);
        // a name can be repeated per endpoint (Prop 65): one entry per name, CAS and endpoint
        let mut e = Entry::new(
            &format!(
                "{} {}",
                if svhc { "candidate" } else { "listing" },
                out.len() + 1
            ),
            name,
            cas,
        );
        e.ec = at(&rec, ec_c).into();
        e.attr = [at(&rec, attr_c), at(&rec, date_c)]
            .iter()
            .filter(|s| !s.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join(" ; ");
        if cas_tokens(cas).is_empty() && e.kind == Kind::Structure {
            e.kind = Kind::Group;
        }
        out.push(e);
    }
    Ok(())
}
