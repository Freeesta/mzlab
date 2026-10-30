//! One reader per source shape. A reader turns the pinned URLs of a list into entries; what is read is recorded in `Ctx` for the provenance and the pins.
//! `eu_water` and `eu_chem` (EUR-Lex XHTML tables), `eu_ppp` (Pesticides API JSON), `us_ecfr` (eCFR XML), `us_epa_html` (EPA pages), `voci` (a hand-transcribed
//! entries file, the act cited per row), `svhc` and `ca_prop65` (the user's own CSV, package C).

use mzlab_core::json::{self, Value};

use super::csvrd::{at, col, guess_sep, Records};
use super::entry::{cas_tokens, Entry, Kind};
use super::fetch::{Cache, Cached, Fetch};
use super::markup::{self, Node};
use super::sources::ListSrc;
use super::table_rd::{self, Cfg};
use super::NormError;

impl<F: Fetch> Loader for Cache<F> {
    fn ensure(&self, url: &str) -> Result<Cached, NormError> {
        Cache::ensure(self, url)
    }
    fn bytes(&self, c: &Cached) -> Result<Vec<u8>, NormError> {
        Cache::bytes(self, c)
    }
}

impl Ctx<'_> {
    /// The text of `url`, recorded as an input.
    pub fn text(&mut self, url: &str) -> Result<String, NormError> {
        let c = self.loader.ensure(url)?;
        let b = self.loader.bytes(&c)?;
        self.inputs.push(Input {
            url: url.into(),
            sha256: c.sha256,
            retrieved: c.retrieved,
            pages: 1,
        });
        Ok(String::from_utf8_lossy(&b).into_owned())
    }
}

/// What a reader reads through: the cache, behind a trait so readers do not depend on the network type.
pub trait Loader {
    fn ensure(&self, url: &str) -> Result<Cached, NormError>;
    fn bytes(&self, c: &Cached) -> Result<Vec<u8>, NormError>;
}

/// One thing read: its URL, the SHA-256 of its bytes (of the page hashes, for a paged source) and the date it was first retrieved.
#[derive(Clone, Debug)]
pub struct Input {
    pub url: String,
    pub sha256: String,
    pub retrieved: String,
    pub pages: usize,
}

pub struct Ctx<'a> {
    pub loader: &'a dyn Loader,
    pub inputs: Vec<Input>,
}

pub fn read(ctx: &mut Ctx, src: &ListSrc) -> Result<Vec<Entry>, NormError> {
    let entries = match src.reader.as_str() {
        "eu_water" | "eu_chem" | "us_epa_html" => tables(ctx, src)?,
        "us_ecfr" => ecfr(ctx, src)?,
        "eu_ppp" => ppp(ctx, src)?,
        "voci" => voci(ctx, src)?,
        "svhc" | "ca_prop65" => import(ctx, src)?,
        other => {
            return Err(NormError::parse(
                &src.id,
                &format!("no reader called {other}"),
            ))
        }
    };
    if entries.is_empty() {
        return Err(NormError::parse(&src.id, "the source gave no entries"));
    }
    Ok(entries)
}

fn tables(ctx: &mut Ctx, src: &ListSrc) -> Result<Vec<Entry>, NormError> {
    let cfg = Cfg::from_params(&src.params);
    let mut out = Vec::new();
    for u in &src.urls {
        let root = markup::parse(&ctx.text(u)?);
        out.extend(table_rd::read(&root, &cfg));
    }
    Ok(out)
}

/// The first element that has a `head` child containing `needle`.
fn section<'a>(n: &'a Node, needle: &str) -> Option<&'a Node> {
    for k in &n.kids {
        if k.kids
            .iter()
            .any(|h| h.name == "head" && h.flat().to_lowercase().contains(needle))
        {
            return Some(k);
        }
        if let Some(s) = section(k, needle) {
            return Some(s);
        }
    }
    None
}

fn ecfr(ctx: &mut Ctx, src: &ListSrc) -> Result<Vec<Entry>, NormError> {
    let p = &src.params;
    let needle = p
        .get("section")
        .and_then(Value::str)
        .unwrap_or("")
        .to_lowercase();
    let lines = p.get("mode").and_then(Value::str) == Some("lines");
    let prefix = p
        .get("prefix")
        .and_then(Value::str)
        .unwrap_or("")
        .to_string();
    let cfg = Cfg::from_params(p);
    let mut out = Vec::new();
    for u in &src.urls {
        let root = markup::parse(&ctx.text(u)?);
        let part = if needle.is_empty() {
            Some(&root)
        } else {
            section(&root, &needle)
        };
        let Some(part) = part else { continue };
        let blocks: Vec<&Node> = ["fp-1", "fp-2", "p"]
            .iter()
            .flat_map(|n| part.find_all(n))
            .collect();
        if !lines {
            out.extend(table_rd::read(part, &cfg));
            continue;
        }
        // "12. Name" or "012 Name": a number, then the name; footnote marks are `su` elements, skipped by `lines`
        for n in blocks {
            let t = n.lines(&["su", "ftref", "ftnt"]).join(" ");
            let digits: String = t.chars().take_while(char::is_ascii_digit).collect();
            let rest = t[digits.len()..].trim_start_matches(['.', ' ']).trim();
            if digits.is_empty() || rest.is_empty() {
                continue;
            }
            let num = digits.trim_start_matches('0');
            out.push(Entry::new(&format!("{prefix}, no. {num}"), rest, ""));
        }
    }
    Ok(out)
}

fn text_of(v: Option<&Value>) -> String {
    match v {
        Some(Value::Str(s)) => s.trim().to_string(),
        Some(Value::Num(n)) => json::fmt_g(*n),
        _ => String::new(),
    }
}

/// The Pesticides API: pages joined by `nextLink`; the pin is the hash of the page hashes.
fn ppp(ctx: &mut Ctx, src: &ListSrc) -> Result<Vec<Entry>, NormError> {
    let first = src
        .urls
        .first()
        .ok_or_else(|| NormError::parse(&src.id, "no url"))?
        .clone();
    let (mut url, mut hashes, mut out) = (Some(first.clone()), String::new(), Vec::new());
    let mut retrieved = String::new();
    let mut pages = 0;
    while let Some(u) = url.take() {
        let c = ctx.loader.ensure(&u)?;
        let v = json::parse(&ctx.loader.bytes(&c)?)
            .map_err(|e| NormError::parse(&src.id, &e.to_string()))?;
        hashes.push_str(&c.sha256);
        hashes.push('\n');
        if retrieved.is_empty() {
            retrieved = c.retrieved.clone();
        }
        pages += 1;
        for r in v.get("value").map(Value::arr).unwrap_or(&[]) {
            let name = text_of(r.get("substance_name"));
            let cas = text_of(r.get("cas_number"));
            if name.is_empty() {
                continue;
            }
            let mut e = Entry::new(
                &format!("substance {}", text_of(r.get("substance_id"))),
                &name,
                &cas,
            );
            let yes = |k: &str| text_of(r.get(k)) == "Yes";
            if yes("as_micro_org") {
                e.kind = Kind::Uvcb;
            } else if yes("as_is_group") {
                e.kind = Kind::Group;
            }
            let mut attr = vec![format!("status={}", text_of(r.get("substance_status")))];
            for (k, label) in [
                ("approval_date", "approved"),
                ("expiry_date", "expires"),
                ("substance_category", "category"),
            ] {
                let t = text_of(r.get(k));
                if !t.is_empty() {
                    attr.push(format!("{label}={t}"));
                }
            }
            if yes("candidate_for_substitution") {
                attr.push("candidate for substitution".into());
            }
            e.attr = attr.join(" ; ");
            out.push(e);
        }
        url = v.get("nextLink").and_then(Value::str).map(String::from);
    }
    let sha256 = mzlab_core::budget::sha256_hex(hashes.as_bytes());
    ctx.inputs.push(Input {
        url: first,
        sha256,
        retrieved,
        pages,
    });
    Ok(out)
}

fn kind_of(word: &str) -> Option<Kind> {
    match word {
        "" | "structure" => Some(Kind::Structure),
        "group" => Some(Kind::Group),
        "uvcb" => Some(Kind::Uvcb),
        "inorganic" => Some(Kind::Inorganic),
        _ => None,
    }
}

/// A hand-transcribed entries file: `entry_ref;name;cas;ec;attr;kind;group_entry;act`, the act cited on every row; an optional `inchikey` column holds a structure the act itself gives. Lines starting with `#` are comments.
fn voci(ctx: &mut Ctx, src: &ListSrc) -> Result<Vec<Entry>, NormError> {
    let mut out = Vec::new();
    for u in &src.urls {
        let text = ctx.text(u)?;
        let body: String = text
            .lines()
            .filter(|l| !l.starts_with('#'))
            .collect::<Vec<_>>()
            .join("\n");
        let mut it = Records::new(body.as_bytes(), b';');
        let head = it.next().unwrap_or_default();
        let c = |n: &str| col(&head, &[n]);
        let (r, n, cas, ec, attr, kind, grp, act, key) = (
            c("entry_ref"),
            c("name"),
            c("cas"),
            c("ec"),
            c("attr"),
            c("kind"),
            c("group_entry"),
            c("act"),
            c("inchikey"),
        );
        for (i, rec) in it.enumerate() {
            let name = at(&rec, n);
            if name.is_empty() {
                continue;
            }
            if at(&rec, act).is_empty() {
                return Err(NormError::parse(
                    &src.id,
                    &format!("row {}: the act is not cited", i + 2),
                ));
            }
            let mut e = Entry::new(
                &format!("{}, {}", at(&rec, act), at(&rec, r)),
                name,
                at(&rec, cas),
            );
            e.ec = at(&rec, ec).into();
            e.attr = at(&rec, attr).into();
            e.group_entry = at(&rec, grp).into();
            e.law_key = at(&rec, key).into();
            let k = at(&rec, kind);
            e.kind = kind_of(k)
                .ok_or_else(|| NormError::parse(&src.id, &format!("row {}: kind {k}", i + 2)))?;
            out.push(e);
        }
    }
    Ok(out)
}

/// The user's own CSV of the ECHA Candidate List (`svhc`) or of OEHHA's Proposition 65 list (`ca_prop65`): the same code in the `.wasm` and in the app reads it.
fn import(ctx: &mut Ctx, src: &ListSrc) -> Result<Vec<Entry>, NormError> {
    let svhc = src.reader == "svhc";
    let mut out: Vec<Entry> = Vec::new();
    for u in &src.urls {
        let text = ctx.text(u)?;
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
            return Err(NormError::parse(
                &src.id,
                "the file has no name and CAS columns",
            ));
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
    }
    Ok(out)
}
