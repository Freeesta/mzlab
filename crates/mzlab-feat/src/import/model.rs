//! The one model of the features of an external table (`newTable`, `addFeature` of the reader): the same fields and the same JSON as the page has always read.

use mzlab_core::json::Out;
use std::collections::HashMap;

/// A value of an open record (the annotations are open structures: another program enters without changing the model).
#[derive(Clone, Debug, PartialEq)]
pub enum V {
    Null,
    Num(f64),
    Str(String),
    Bool(bool),
    List(Vec<String>),
}

impl V {
    pub fn opt(v: Option<f64>) -> V {
        v.map_or(V::Null, V::Num)
    }
    pub fn s(v: &str) -> V {
        V::Str(v.to_string())
    }
}

/// An annotation: the keys in the order they were first written.
#[derive(Clone, Debug, Default)]
pub struct Rec(pub Vec<(&'static str, V)>);

impl Rec {
    pub fn set(&mut self, k: &'static str, v: V) {
        match self.0.iter_mut().find(|(x, _)| *x == k) {
            Some(e) => e.1 = v,
            None => self.0.push((k, v)),
        }
    }
    pub fn get(&self, k: &str) -> Option<&V> {
        self.0.iter().find(|(x, _)| *x == k).map(|(_, v)| v)
    }
    pub fn text(&self, k: &str) -> &str {
        match self.get(k) {
            Some(V::Str(s)) => s,
            _ => "",
        }
    }
    pub fn num(&self, k: &str) -> Option<f64> {
        match self.get(k) {
            Some(V::Num(n)) => Some(*n),
            _ => None,
        }
    }
    pub fn flag(&self, k: &str) -> bool {
        matches!(self.get(k), Some(V::Bool(true)))
    }
    pub fn push_note(&mut self, note: &str) {
        match self.0.iter_mut().find(|(x, _)| *x == "notes") {
            Some((_, V::List(l))) => l.push(note.to_string()),
            _ => self.0.push(("notes", V::List(vec![note.to_string()]))),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Ms2 {
    pub mz: Vec<f64>,
    pub it: Vec<f64>,
    /// MGF spectra also carry the precursor and the collision energy.
    pub extra: Option<(Option<f64>, String)>,
}

#[derive(Clone, Debug)]
pub struct Iin {
    pub id: String,
    pub ions: String,
    pub neutral: Option<f64>,
}

#[derive(Clone, Debug)]
pub struct Feature {
    pub key: String,
    pub id: String,
    pub pol: i32,
    pub mz: f64,
    pub rt: f64,
    pub areas: Vec<(String, f64)>,
    pub detected: Option<Vec<String>>,
    pub ms2: Option<Ms2>,
    pub ann: Vec<Rec>,
    pub group: Option<String>,
    pub iin: Option<Iin>,
    pub component: Option<f64>,
    pub n_scans: Option<f64>,
    pub program: String,
}

impl Feature {
    /// The area of a sample (a repeated sample name replaces the earlier area, as the keys of an object do).
    pub fn set_area(&mut self, sample: &str, a: f64) {
        match self.areas.iter_mut().find(|(s, _)| s == sample) {
            Some(e) => e.1 = a,
            None => self.areas.push((sample.to_string(), a)),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Edge {
    pub a: String,
    pub b: String,
    pub kind: String,
    pub score: Option<f64>,
    pub annotation: String,
}

#[derive(Clone, Debug)]
pub struct Table {
    pub program: String,
    pub name: String,
    pub sha256: String,
    pub pol: i32,
    pub features: Vec<Feature>,
    pub by_id: HashMap<String, usize>,
    pub samples: Vec<String>,
    pub edges: Vec<Edge>,
    pub warnings: Vec<String>,
    pub stats: Vec<(&'static str, f64)>,
    pub used: Vec<String>,
    pub ignored: Vec<String>,
}

/// `keyOf`: «POS:12», «NEG:12» or «UNK:12».
pub fn key_of(pol: i32, id: &str) -> String {
    format!(
        "{}:{id}",
        if pol > 0 {
            "POS"
        } else if pol < 0 {
            "NEG"
        } else {
            "UNK"
        }
    )
}

/// `polOf`: 1, -1 or 0 from «pos», «positive», «+», «1» / «neg», «negative», «-», «-1».
pub fn pol_of(v: &str) -> i32 {
    match super::delim::txt(v).to_lowercase().as_str() {
        "pos" | "positive" | "+" | "1" => 1,
        "neg" | "negative" | "-" | "-1" => -1,
        _ => 0,
    }
}

impl Table {
    pub fn new(program: &str, name: &str, pol: i32) -> Table {
        Table {
            program: program.into(),
            name: name.into(),
            sha256: String::new(),
            pol,
            features: vec![],
            by_id: HashMap::new(),
            samples: vec![],
            edges: vec![],
            warnings: vec![],
            stats: vec![],
            used: vec![],
            ignored: vec![],
        }
    }
    pub fn add_feature(&mut self, id: &str, mz: f64, rt: f64) -> &mut Feature {
        let f = Feature {
            key: key_of(self.pol, id),
            id: id.into(),
            pol: self.pol,
            mz,
            rt,
            areas: vec![],
            detected: None,
            ms2: None,
            ann: vec![],
            group: None,
            iin: None,
            component: None,
            n_scans: None,
            program: self.program.clone(),
        };
        self.by_id.insert(id.into(), self.features.len());
        self.features.push(f);
        self.features.last_mut().unwrap()
    }
    pub fn warn(&mut self, msg: String) {
        if self.warnings.len() < 50 {
            self.warnings.push(msg);
        }
    }
    pub fn stat(&mut self, k: &'static str, v: f64) {
        match self.stats.iter_mut().find(|(x, _)| *x == k) {
            Some(e) => e.1 = v,
            None => self.stats.push((k, v)),
        }
    }
    pub fn stat_of(&self, k: &str) -> Option<f64> {
        self.stats.iter().find(|(x, _)| *x == k).map(|(_, v)| *v)
    }
    pub fn feature(&self, id: &str) -> Option<usize> {
        self.by_id.get(id).copied()
    }
}

fn strs(o: &mut Out, a: &[String]) {
    o.raw("[");
    for (i, s) in a.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(s);
    }
    o.raw("]");
}

fn val(o: &mut Out, v: &V) {
    match v {
        V::Null => o.raw("null"),
        V::Num(n) => o.num(*n),
        V::Str(s) => o.str(s),
        V::Bool(b) => o.raw(if *b { "true" } else { "false" }),
        V::List(l) => {
            strs(o, l);
            o
        }
    };
}

fn rec(o: &mut Out, r: &Rec) {
    o.raw("{");
    for (i, (k, v)) in r.0.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(k).raw(":");
        val(o, v);
    }
    o.raw("}");
}

fn feature(o: &mut Out, f: &Feature) {
    o.raw("{\"key\":")
        .str(&f.key)
        .raw(",\"id\":")
        .str(&f.id)
        .raw(",\"pol\":")
        .int(f.pol as i64);
    o.raw(",\"mz\":")
        .num(f.mz)
        .raw(",\"rt\":")
        .num(f.rt)
        .raw(",\"areas\":{");
    for (i, (s, a)) in f.areas.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(s).raw(":").num(*a);
    }
    o.raw("},\"detected\":");
    match &f.detected {
        Some(d) => strs(o, d),
        None => {
            o.raw("null");
        }
    }
    o.raw(",\"ms2\":");
    match &f.ms2 {
        Some(m) => {
            o.raw("{\"mz\":")
                .floats(&m.mz)
                .raw(",\"it\":")
                .floats(&m.it);
            if let Some((p, ce)) = &m.extra {
                o.raw(",\"prec\":").opt(*p).raw(",\"ce\":").str(ce);
            }
            o.raw("}");
        }
        None => {
            o.raw("null");
        }
    }
    o.raw(",\"ann\":[");
    for (i, a) in f.ann.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" });
        rec(o, a);
    }
    o.raw("],\"group\":");
    match &f.group {
        Some(g) => o.str(g),
        None => o.raw("null"),
    };
    o.raw(",\"iin\":");
    match &f.iin {
        Some(i) => o
            .raw("{\"id\":")
            .str(&i.id)
            .raw(",\"ions\":")
            .str(&i.ions)
            .raw(",\"neutral\":")
            .opt(i.neutral)
            .raw("}"),
        None => o.raw("null"),
    };
    o.raw(",\"component\":")
        .opt(f.component)
        .raw(",\"nScans\":")
        .opt(f.n_scans)
        .raw(",\"program\":")
        .str(&f.program)
        .raw("}");
}

/// The table as JSON: the shape of the model that the lists, the levels and the exports read.
pub fn table_json(t: &Table) -> String {
    let mut o = Out::default();
    o.raw("{\"program\":")
        .str(&t.program)
        .raw(",\"file\":{\"name\":")
        .str(&t.name)
        .raw(",\"sha256\":")
        .str(&t.sha256);
    o.raw("},\"pol\":").int(t.pol as i64).raw(",\"features\":[");
    for (i, f) in t.features.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" });
        feature(&mut o, f);
    }
    o.raw("],\"samples\":");
    strs(&mut o, &t.samples);
    o.raw(",\"edges\":[");
    for (i, e) in t.edges.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"a\":")
            .str(&e.a)
            .raw(",\"b\":")
            .str(&e.b)
            .raw(",\"type\":")
            .str(&e.kind);
        o.raw(",\"score\":")
            .opt(e.score)
            .raw(",\"annotation\":")
            .str(&e.annotation)
            .raw("}");
    }
    o.raw("],\"warnings\":");
    strs(&mut o, &t.warnings);
    o.raw(",\"stats\":{");
    for (i, (k, v)) in t.stats.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(k).raw(":").num(*v);
    }
    o.raw("},\"used\":");
    strs(&mut o, &t.used);
    o.raw(",\"ignored\":");
    strs(&mut o, &t.ignored);
    o.raw("}");
    o.0
}
