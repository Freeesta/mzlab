//! The one reader of the header of an mzML spectrum or chromatogram, shared by every mzML read path: its terms (spectrum, scan list and scans, in file
//! order), its precursors in file order (isolation window, first selected ion, activation), its products (never a precursor) and the terms of its binary
//! arrays. `referenceableParamGroupRef` replays the group's params where it stands. Fed by tag events (`Reader::start`, `end`, `text`), from the tag
//! scanner (`tags`, for header text) or from a streaming XML parser; names are local (no namespace prefix).

use std::borrow::Cow;

mod groups;
mod immediate;
pub mod tags;

pub use groups::Groups;
pub use immediate::immediate;

/// One cvParam (or userParam: `acc` empty, the user name in `name`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Term {
    pub acc: String,
    pub name: String,
    pub value: String,
    pub unit_acc: String,
    pub unit_name: String,
}

impl Term {
    /// One pass over the attributes; the first of a repeated one wins, as a lookup would give.
    fn of(t: &[(&str, Cow<str>)], user: bool) -> Term {
        let mut out = Term::default();
        let mut seen = [false; 5];
        for (k, v) in t {
            let (i, to) = match *k {
                "accession" if !user => (0, &mut out.acc),
                "name" => (1, &mut out.name),
                "value" => (2, &mut out.value),
                "unitAccession" => (3, &mut out.unit_acc),
                "unitName" => (4, &mut out.unit_name),
                _ => continue,
            };
            if !seen[i] {
                seen[i] = true;
                to.push_str(v);
            }
        }
        out
    }
    #[inline(never)]
    pub fn float(&self) -> Option<f64> {
        self.value.trim().parse().ok()
    }
}

/// Terms in file order with lookups by accession (the first occurrence wins).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Terms(pub Vec<Term>);

impl Terms {
    #[inline(never)]
    pub fn get(&self, acc: &str) -> Option<&Term> {
        self.0.iter().find(|t| t.acc == acc)
    }
    pub fn has(&self, acc: &str) -> bool {
        self.get(acc).is_some()
    }
    #[inline(never)]
    pub fn float(&self, acc: &str) -> Option<f64> {
        self.get(acc).and_then(Term::float)
    }
    /// The value of the first userParam named `name`.
    #[inline(never)]
    pub fn user(&self, name: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|t| t.acc.is_empty() && t.name == name)
            .map(|t| t.value.as_str())
    }
}

/// One `<precursor>` (or `<product>`): its `spectrumRef`, the terms of its isolation window, of its first selected ion and of its activation.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stage {
    pub refer: Option<String>,
    pub window: Terms,
    pub ion: Terms,
    pub activation: Terms,
    ions: usize,
}

impl Stage {
    /// A stage read from loose terms (a selected ion or target written outside a `<precursor>`).
    #[inline(never)]
    pub fn loose(t: &Terms) -> Stage {
        Stage {
            window: t.clone(),
            ion: t.clone(),
            activation: t.clone(),
            ..Stage::default()
        }
    }
    #[inline(never)]
    pub fn target(&self) -> Option<f64> {
        self.window.float("MS:1000827")
    }
    /// The low and high edges of the isolation window (target minus and plus its offsets).
    #[inline(never)]
    pub fn edges(&self) -> (Option<f64>, Option<f64>) {
        let t = self.target();
        (
            t.zip(self.window.float("MS:1000828")).map(|(t, o)| t - o),
            t.zip(self.window.float("MS:1000829")).map(|(t, o)| t + o),
        )
    }
    #[inline(never)]
    pub fn selected(&self) -> Option<f64> {
        self.ion.float("MS:1000744")
    }
    pub fn charge(&self) -> Option<f64> {
        self.ion.float("MS:1000041")
    }
    pub fn intensity(&self) -> Option<f64> {
        self.ion.float("MS:1000042")
    }
    pub fn energy(&self) -> Option<f64> {
        self.activation.float("MS:1000045")
    }
    /// The short name of the dissociation method (`activations::ATTIVAZIONI` order).
    pub fn method(&self) -> Option<&'static str> {
        crate::activations::ATTIVAZIONI
            .iter()
            .find(|a| self.activation.has(a.1))
            .map(|a| a.0)
    }
}

/// One `<binaryDataArray>`: its terms, `arrayLength` and the text of its `<binary>` (only when the reader keeps it).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ArrayHead {
    pub terms: Terms,
    pub length: Option<usize>,
    pub text: String,
}

/// The unit of a retention time (`MS:1000016` requires one; a missing unit is read as minutes and said so).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RtUnit {
    Minute,
    Second,
    Millisecond,
    Hour,
    Missing,
}

impl RtUnit {
    pub fn of(t: &Term) -> RtUnit {
        match (t.unit_acc.as_str(), t.unit_name.as_str()) {
            ("UO:0000031", _) | (_, "minute") => RtUnit::Minute,
            ("UO:0000010", _) | (_, "second") => RtUnit::Second,
            ("UO:0000028", _) | (_, "millisecond") => RtUnit::Millisecond,
            ("UO:0000032", _) | (_, "hour") => RtUnit::Hour,
            _ => RtUnit::Missing,
        }
    }
    /// Minutes per unit.
    pub fn minutes(self) -> f64 {
        match self {
            RtUnit::Minute | RtUnit::Missing => 1.0,
            RtUnit::Second => 1.0 / 60.0,
            RtUnit::Millisecond => 1.0 / 60_000.0,
            RtUnit::Hour => 60.0,
        }
    }
}

/// The header of one spectrum or chromatogram.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Head {
    pub chromatogram: bool,
    pub id: String,
    pub index: Option<usize>,
    pub length: Option<usize>,
    /// The terms of the element itself, of its scan list and of its scans (and their windows), in file order.
    pub terms: Terms,
    pub scans: usize,
    /// `instrumentConfigurationRef` of the first scan.
    pub ic_ref: Option<String>,
    pub precursors: Vec<Stage>,
    pub products: Vec<Stage>,
    pub arrays: Vec<ArrayHead>,
}

impl Head {
    /// The level (`MS:1000511`), 1 when absent.
    #[inline(never)]
    pub fn level(&self) -> u8 {
        // a level is a small integer; `as` saturates
        self.terms.float("MS:1000511").map_or(1, |v| v as u8)
    }
    /// +1, -1 or 0 (not said).
    #[inline(never)]
    pub fn polarity(&self) -> i8 {
        if self.terms.has("MS:1000130") {
            1
        } else if self.terms.has("MS:1000129") {
            -1
        } else {
            0
        }
    }
    #[inline(never)]
    pub fn profile(&self) -> bool {
        self.terms.has("MS:1000128")
    }
    /// The retention time of the first scan (its first `MS:1000016`) and its unit.
    pub fn rt(&self) -> Option<(f64, RtUnit)> {
        let t = self.terms.get("MS:1000016")?;
        Some((t.float().unwrap_or(0.0), RtUnit::of(t)))
    }
    /// The retention time in minutes (0 when absent).
    #[inline(never)]
    pub fn rt_minutes(&self) -> f64 {
        self.rt().map_or(0.0, |(v, u)| v * u.minutes())
    }
    #[inline(never)]
    pub fn filter(&self) -> &str {
        self.terms
            .get("MS:1000512")
            .map_or("", |t| t.value.as_str())
    }
}

/// Where a param goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Ctx {
    Item,
    Scan,
    Precursor,
    Product,
    Window,
    Ion,
    Activation,
    Array,
    Binary,
    Group,
    Other,
}

/// The terms kept in one list (an item, an array, a stage part, a group) and the precursors or products kept per item: lookups take the first
/// occurrence, so a file that repeats a param millions of times costs no more memory than a real header.
const MAX_TERMS: usize = 4096;
const MAX_STAGES: usize = 64;

/// The builder: feed it the tags of a file (or of a header text) and take each spectrum or chromatogram header at its end tag.
#[derive(Default)]
pub struct Reader<'g> {
    /// The groups known so far (borrowed from the file header, or collected while reading).
    pub groups: Cow<'g, Groups>,
    /// Keep the text of `<binary>` in `ArrayHead::text`.
    pub keep_binary: bool,
    stack: Vec<Ctx>,
    cur: Option<Head>,
    group: Option<(String, Vec<Term>)>,
}

impl<'g> Reader<'g> {
    pub fn new(groups: &'g Groups) -> Reader<'g> {
        Reader {
            groups: Cow::Borrowed(groups),
            ..Reader::default()
        }
    }

    /// An open (or empty, `empty`) element `name` with its attributes.
    pub fn start(&mut self, name: &str, attrs: &[(&str, Cow<str>)], empty: bool) {
        let get = |k: &str| attrs.iter().find(|(n, _)| *n == k).map(|(_, v)| v.as_ref());
        let top = self.stack.last().copied();
        let ctx = match name {
            "spectrum" | "chromatogram" => {
                self.cur = Some(Head {
                    chromatogram: name == "chromatogram",
                    id: get("id").unwrap_or("").to_string(),
                    index: get("index").and_then(|v| v.trim().parse().ok()),
                    length: get("defaultArrayLength").and_then(|v| v.trim().parse().ok()),
                    ..Head::default()
                });
                Ctx::Item
            }
            "referenceableParamGroup" => {
                self.group = Some((get("id").unwrap_or("").to_string(), Vec::new()));
                Ctx::Group
            }
            "cvParam" | "userParam" => {
                let t = Term::of(attrs, name == "userParam");
                self.put(t);
                Ctx::Other
            }
            "referenceableParamGroupRef" => {
                if let Some(g) = get("ref").and_then(|r| self.groups.get(r)) {
                    for t in g.clone() {
                        self.put(t);
                    }
                }
                Ctx::Other
            }
            _ if self.cur.is_none() => Ctx::Other,
            _ => self.item_tag(name, attrs, top),
        };
        if !empty {
            self.stack.push(ctx);
        } else if ctx == Ctx::Group {
            self.end_group();
        }
    }

    /// An element inside a spectrum or chromatogram (scans, precursors, products, arrays).
    #[inline(never)]
    fn item_tag(&mut self, name: &str, attrs: &[(&str, Cow<str>)], top: Option<Ctx>) -> Ctx {
        let get = |k: &str| attrs.iter().find(|(n, _)| *n == k).map(|(_, v)| v.as_ref());
        match name {
            "scanList" if top == Some(Ctx::Item) => Ctx::Scan,
            "scan" if top == Some(Ctx::Scan) => {
                let h = self.cur.as_mut().expect("in an item");
                if h.scans == 0 {
                    h.ic_ref = get("instrumentConfigurationRef").map(String::from);
                }
                h.scans += 1;
                Ctx::Scan
            }
            "scanWindowList" | "scanWindow" if top == Some(Ctx::Scan) => Ctx::Scan,
            "precursor" | "product" => {
                let h = self.cur.as_mut().expect("in an item");
                let st = Stage {
                    refer: get("spectrumRef").map(String::from),
                    ..Stage::default()
                };
                if name == "precursor" {
                    if h.precursors.len() < MAX_STAGES {
                        h.precursors.push(st);
                    }
                    Ctx::Precursor
                } else {
                    if h.products.len() < MAX_STAGES {
                        h.products.push(st);
                    }
                    Ctx::Product
                }
            }
            "isolationWindow" if matches!(top, Some(Ctx::Precursor | Ctx::Product)) => Ctx::Window,
            "selectedIon" if top == Some(Ctx::Precursor) => {
                let st = self.stage().expect("in a precursor");
                st.ions += 1;
                if st.ions == 1 {
                    Ctx::Ion
                } else {
                    Ctx::Other
                }
            }
            "activation" if top == Some(Ctx::Precursor) => Ctx::Activation,
            "selectedIonList" if top == Some(Ctx::Precursor) => Ctx::Precursor,
            "binaryDataArray" => {
                let h = self.cur.as_mut().expect("in an item");
                h.arrays.push(ArrayHead {
                    length: get("arrayLength").and_then(|v| v.trim().parse().ok()),
                    ..ArrayHead::default()
                });
                Ctx::Array
            }
            "binary" if top == Some(Ctx::Array) => Ctx::Binary,
            _ => Ctx::Other,
        }
    }

    /// The current precursor or product.
    fn stage(&mut self) -> Option<&mut Stage> {
        let h = self.cur.as_mut()?;
        let in_product = self
            .stack
            .iter()
            .rev()
            .find(|c| matches!(c, Ctx::Precursor | Ctx::Product))
            == Some(&Ctx::Product);
        if in_product {
            h.products.last_mut()
        } else {
            h.precursors.last_mut()
        }
    }

    /// Params where the reader stands (a cvParam, a userParam or a replayed group).
    fn put(&mut self, t: Term) {
        let ctx = self.stack.last().copied();
        let to = match ctx {
            Some(Ctx::Group) => self.group.as_mut().map(|(_, g)| g),
            Some(Ctx::Item | Ctx::Scan) => self.cur.as_mut().map(|h| &mut h.terms.0),
            Some(Ctx::Array) => self
                .cur
                .as_mut()
                .and_then(|h| h.arrays.last_mut())
                .map(|a| &mut a.terms.0),
            Some(c @ (Ctx::Window | Ctx::Ion | Ctx::Activation)) => self.stage().map(|st| {
                &mut match c {
                    Ctx::Window => &mut st.window,
                    Ctx::Ion => &mut st.ion,
                    _ => &mut st.activation,
                }
                .0
            }),
            _ => None,
        };
        if let Some(to) = to.filter(|to| to.len() < MAX_TERMS) {
            to.push(t);
        }
    }

    fn end_group(&mut self) {
        if let Some((id, g)) = self.group.take() {
            self.groups.to_mut().insert(id, g);
        }
    }

    /// Whether the reader stands inside a `<binary>` (where only `keep_binary` makes text count).
    pub fn in_binary(&self) -> bool {
        self.stack.last() == Some(&Ctx::Binary)
    }

    /// Character data: kept inside `<binary>` when `keep_binary`.
    pub fn text(&mut self, t: &str) {
        if self.keep_binary
            && self.stack.last() == Some(&Ctx::Binary)
            && let Some(a) = self.cur.as_mut().and_then(|h| h.arrays.last_mut())
        {
            a.text.push_str(t);
        }
    }

    /// The end of element `name`: the finished header at `</spectrum>` or `</chromatogram>`.
    pub fn end(&mut self, name: &str) -> Option<Head> {
        let ctx = self.stack.pop();
        match ctx {
            Some(Ctx::Group) => self.end_group(),
            Some(Ctx::Item) if matches!(name, "spectrum" | "chromatogram") => {
                return self.cur.take();
            }
            _ => {}
        }
        None
    }

    /// The header being read (a text cut before its end tag).
    pub fn take(&mut self) -> Option<Head> {
        self.stack.clear();
        self.cur.take()
    }

    /// Feeds the tags of `text`.
    pub fn feed(&mut self, text: &str, mut each: impl FnMut(Head)) {
        for it in tags::items(text) {
            match it {
                tags::Item::Text(t) => self.text(&t),
                tags::Item::Tag(t) => match t.kind {
                    tags::Kind::Open => self.start(t.name, &t.attrs, false),
                    tags::Kind::Empty => self.start(t.name, &t.attrs, true),
                    tags::Kind::Close => {
                        if let Some(h) = self.end(t.name) {
                            each(h);
                        }
                    }
                },
            }
        }
    }
}

/// The groups defined in a file header text.
pub fn groups(header: &str) -> Groups {
    let mut r = Reader::default();
    r.feed(header, |_| {});
    r.groups.into_owned()
}

/// The header of the one spectrum or chromatogram in `text` (cut anywhere after its start tag), with the file's `groups`.
pub fn parse(text: &str, groups: &Groups) -> Option<Head> {
    let mut r = Reader::new(groups);
    let mut out = None;
    r.feed(text, |h| out = out.take().or(Some(h)));
    out.or_else(|| r.take())
}

#[cfg(test)]
mod tests;
