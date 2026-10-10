//! Streaming readers of spectral libraries: MSP, MGF, MassBank records (`.txt`), JSON arrays (MoNA, GNPS) and JSON lines (mzMine).
//! `Parser::push` takes the file in any chunking (even in the middle of a character) and calls the callback for every usable record.
//! A record without a precursor or without peaks is counted and dropped, never an error; text fields are cut at `MAX_FIELD_BYTES`.

use crate::error::{Error, Result};
use crate::spec::{Meta, Spectrum, MAX_FIELD_BYTES, MAX_RAW_PEAKS};
use serde_json::Value;
use std::collections::HashMap;

/// A text line longer than this is skipped (counted as broken); an unbounded line would eat the memory.
const MAX_LINE: usize = 16 << 20;
/// A JSON object larger than this is skipped (counted as broken).
const MAX_OBJECT: usize = 32 << 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    Msp,
    Mgf,
    MassBank,
    Json,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    pub read: u64,
    pub dropped: u64,
    pub no_prec: u64,
    pub no_peaks: u64,
    pub broken: u64,
}

/// Format by extension, otherwise by the first bytes. `lib.err.unsupported`: commercial binary libraries; `lib.err.unknown`: anything else.
pub fn detect(name: &str, head: &[u8]) -> Result<Format> {
    let n = name.to_ascii_lowercase();
    let ext_is = |e: &[&str]| e.iter().any(|x| n.ends_with(x));
    if ext_is(&[".lib", ".mzvault", ".sqlite", ".db", ".nist"]) {
        return Err(Error::new("lib.err.unsupported"));
    }
    if ext_is(&[".msp"]) {
        return Ok(Format::Msp);
    }
    if ext_is(&[".mgf"]) {
        return Ok(Format::Mgf);
    }
    if ext_is(&[".json", ".jsonl", ".ndjson"]) {
        return Ok(Format::Json);
    }
    let h = String::from_utf8_lossy(&head[..head.len().min(8192)]).into_owned();
    let first = h.trim_start_matches('\u{feff}').trim_start();
    if h.to_ascii_uppercase().contains("BEGIN IONS") {
        Ok(Format::Mgf)
    } else if first.starts_with('{') || first.starts_with('[') {
        Ok(Format::Json)
    } else if h.lines().any(|l| {
        l.trim_start()
            .to_ascii_uppercase()
            .starts_with("ACCESSION:")
    }) {
        Ok(Format::MassBank)
    } else if h.lines().any(|l| {
        let l = l.trim().to_ascii_lowercase();
        l.starts_with("name:") || l.starts_with("name :")
    }) {
        Ok(Format::Msp)
    } else {
        Err(Error::new("lib.err.unknown"))
    }
}

// ---------------------------------------------------------------- numbers

/// The next number (the JS regex `[-+]?(\d+\.?\d*|\.\d+)([eE][-+]?\d+)?`) at or after `*pos`.
fn next_number(s: &[u8], pos: &mut usize) -> Option<f64> {
    find_number(s, pos).map(|(_, v)| v)
}

/// Like `next_number`, also giving the start of the match.
fn find_number(s: &[u8], pos: &mut usize) -> Option<(usize, f64)> {
    let n = s.len();
    let mut i = *pos;
    while i < n {
        let start = i;
        let mut j = i;
        if j < n && (s[j] == b'+' || s[j] == b'-') {
            j += 1;
        }
        let d0 = j;
        while j < n && s[j].is_ascii_digit() {
            j += 1;
        }
        let mut ok = j > d0;
        if ok {
            if j < n && s[j] == b'.' {
                j += 1;
                while j < n && s[j].is_ascii_digit() {
                    j += 1;
                }
            }
        } else if j < n && s[j] == b'.' && j + 1 < n && s[j + 1].is_ascii_digit() {
            j += 1;
            while j < n && s[j].is_ascii_digit() {
                j += 1;
            }
            ok = true;
        }
        if ok {
            if j < n && (s[j] == b'e' || s[j] == b'E') {
                let mut k = j + 1;
                if k < n && (s[k] == b'+' || s[k] == b'-') {
                    k += 1;
                }
                let e0 = k;
                while k < n && s[k].is_ascii_digit() {
                    k += 1;
                }
                if k > e0 {
                    j = k;
                }
            }
            *pos = j;
            return std::str::from_utf8(&s[start..j])
                .ok()
                .and_then(|t| t.parse().ok())
                .map(|v| (start, v));
        }
        i += 1;
    }
    None
}

/// JS `parseFloat` after turning a comma into a point: a numeric prefix, else NaN.
fn parse_float(s: &str) -> f64 {
    let t = s.trim_start().replace(',', ".");
    let b = t.as_bytes();
    let mut pos = 0;
    let before = b.len();
    match next_number(b, &mut pos) {
        Some(v)
            if t.starts_with(|c: char| c == '+' || c == '-' || c == '.' || c.is_ascii_digit())
                && pos <= before =>
        {
            v
        }
        _ => f64::NAN,
    }
}

// ---------------------------------------------------------------- one record

/// Normalised key: lower case without spaces, underscores and hyphens.
fn key_of(k: &str) -> String {
    k.chars()
        .filter(|c| !c.is_whitespace() && *c != '_' && *c != '-')
        .flat_map(|c| c.to_lowercase())
        .collect()
}

fn limit(s: &str) -> String {
    let t = s.trim();
    if t.len() <= MAX_FIELD_BYTES {
        return t.to_string();
    }
    let mut e = MAX_FIELD_BYTES;
    while !t.is_char_boundary(e) {
        e -= 1;
    }
    t[..e].to_string()
}

fn pol_from(mode: &str, adduct: &str, charge: &str) -> i8 {
    let m = mode.trim();
    if m.starts_with(['p', 'P']) {
        return 1;
    }
    if m.starts_with(['n', 'N']) {
        return -1;
    }
    // adduct ending in «]n+» / «]n-»
    let a = adduct.trim_end();
    if let Some(sign) = a.chars().last().filter(|c| *c == '+' || *c == '-') {
        let body = &a[..a.len() - 1];
        let body = body.trim_end_matches(|c: char| c.is_ascii_digit());
        if body.ends_with(']') {
            return if sign == '+' { 1 } else { -1 };
        }
    }
    match charge.chars().find(|c| *c == '+' || *c == '-') {
        Some('+') => 1,
        Some(_) => -1,
        None => 0,
    }
}

#[derive(Default)]
struct Rec {
    f: HashMap<String, String>,
    mz: Vec<f64>,
    it: Vec<f64>,
    extra_peaks: bool,
}

impl Rec {
    fn field(&mut self, k: String, v: &str) {
        self.f.entry(k).or_insert_with(|| v.to_string());
    }

    fn peak(&mut self, m: f64, i: f64) {
        if m.is_finite() && i.is_finite() && m > 0.0 && i > 0.0 {
            if self.mz.len() < MAX_RAW_PEAKS {
                self.mz.push(m);
                self.it.push(i);
            } else {
                self.extra_peaks = true;
            }
        }
    }

    /// Peak text: segments split by «;», the first two numbers of each (JS `LibParser.peak`).
    fn peaks_text(&mut self, line: &str) {
        for seg in line.split(';') {
            let b = seg.as_bytes();
            let mut pos = 0;
            if let (Some(m), Some(i)) = (next_number(b, &mut pos), next_number(b, &mut pos)) {
                self.peak(m, i);
            }
        }
    }

    fn get(&self, keys: &[&str]) -> Option<&str> {
        keys.iter().find_map(|k| self.f.get(*k).map(|s| s.as_str()))
    }
}

/// Turns a record into a spectrum or counts why not.
fn finish(c: Rec, stats: &mut Stats) -> Option<Spectrum> {
    let prec = parse_float(
        c.get(&["precursormz", "precursor", "precursorm/z", "pepmass"])
            .unwrap_or(""),
    );
    if prec.is_nan() || prec <= 0.0 {
        stats.dropped += 1;
        stats.no_prec += 1;
        return None;
    }
    if c.mz.is_empty() {
        stats.dropped += 1;
        stats.no_peaks += 1;
        return None;
    }
    let adduct = c
        .get(&["precursortype", "adduct", "ionadduct"])
        .unwrap_or("");
    let pol = pol_from(
        c.get(&["ionmode", "polarity", "ionizationmode"])
            .unwrap_or(""),
        adduct,
        c.get(&["charge"]).unwrap_or(""),
    );
    let g = |keys: &[&str]| limit(c.get(keys).unwrap_or(""));
    let meta = Meta {
        name: g(&["name", "title", "compoundname", "chname", "recordtitle"]),
        adduct: limit(adduct),
        ce: g(&["collisionenergy", "ce"]),
        instrument: g(&["instrumenttype", "instrument"]),
        formula: g(&["formula", "molformula", "molecularformula"]),
        inchikey: g(&["inchikey", "inchikeysmiles"]),
        smiles: g(&["smiles", "isomericsmiles"]),
        accession: g(&["accession", "spectrumid", "id"]),
        authors: g(&["authors", "author"]),
        license: g(&["license", "licence"]),
    };
    stats.read += 1;
    Some(Spectrum {
        prec,
        pol,
        mz: c.mz,
        it: c.it,
        meta,
    })
}

// ---------------------------------------------------------------- the parser

pub struct Parser<F: FnMut(Spectrum)> {
    fmt: Format,
    cb: F,
    pub stats: Stats,
    line: Vec<u8>,
    skipping: bool,
    cur: Option<Rec>,
    in_peaks: bool,
    // JSON splitting
    depth: u32,
    in_str: bool,
    esc: bool,
    obj: Vec<u8>,
    obj_skip: bool,
}

impl<F: FnMut(Spectrum)> Parser<F> {
    pub fn new(fmt: Format, cb: F) -> Self {
        Parser {
            fmt,
            cb,
            stats: Stats::default(),
            line: Vec::new(),
            skipping: false,
            cur: None,
            in_peaks: false,
            depth: 0,
            in_str: false,
            esc: false,
            obj: Vec::new(),
            obj_skip: false,
        }
    }

    pub fn format(&self) -> Format {
        self.fmt
    }

    pub fn push(&mut self, data: &[u8]) {
        if self.fmt == Format::Json {
            self.push_json(data);
            return;
        }
        let mut start = 0;
        for (i, &b) in data.iter().enumerate() {
            if b == b'\n' {
                self.add_line_bytes(&data[start..i]);
                self.end_line();
                start = i + 1;
            }
        }
        self.add_line_bytes(&data[start..]);
    }

    fn add_line_bytes(&mut self, part: &[u8]) {
        if self.skipping {
            return;
        }
        if self.line.len() + part.len() > MAX_LINE {
            self.skipping = true;
            self.line.clear();
            self.stats.broken += 1;
            return;
        }
        self.line.extend_from_slice(part);
    }

    fn end_line(&mut self) {
        if self.skipping {
            self.skipping = false;
            return;
        }
        let bytes = std::mem::take(&mut self.line);
        let s = String::from_utf8_lossy(&bytes);
        let s = s.strip_suffix('\r').unwrap_or(&s);
        let s = s.trim_start_matches('\u{feff}');
        match self.fmt {
            Format::Msp => self.msp(s),
            Format::Mgf => self.mgf(s),
            Format::MassBank => self.massbank(s),
            Format::Json => {}
        }
        self.line = bytes;
        self.line.clear();
    }

    pub fn end(&mut self) {
        if self.fmt == Format::Json {
            if self.depth > 0 || !self.obj.is_empty() {
                self.stats.dropped += 1;
                self.stats.broken += 1;
                self.obj.clear();
                self.depth = 0;
            }
            return;
        }
        if !self.line.is_empty() && !self.skipping {
            self.end_line();
        }
        if self.cur.is_some() {
            match self.fmt {
                Format::Mgf => self.drop_broken(),
                _ => self.emit(),
            }
        }
    }

    fn drop_broken(&mut self) {
        self.stats.dropped += 1;
        self.stats.broken += 1;
        self.cur = None;
        self.in_peaks = false;
    }

    fn emit(&mut self) {
        self.in_peaks = false;
        if let Some(c) = self.cur.take() {
            if let Some(s) = finish(c, &mut self.stats) {
                (self.cb)(s);
            }
        }
    }

    // ------------------------------------------------ MSP
    fn msp(&mut self, l: &str) {
        let t = l.trim();
        if t.is_empty() {
            if self.cur.is_some() {
                self.emit();
            }
            return;
        }
        if let Some((k, v)) = split_key(t, ':') {
            let key = key_of(k);
            if !self.in_peaks || key == "name" {
                // a «Name:» line always starts a new record, even without the blank line
                if self.cur.is_some()
                    && key == "name"
                    && (self.in_peaks
                        || self.cur.as_ref().is_some_and(|c| c.f.contains_key("name")))
                {
                    self.emit();
                }
                let c = self.cur.get_or_insert_with(Rec::default);
                if key == "numpeaks" {
                    self.in_peaks = true;
                    return;
                }
                c.field(key, v.trim());
                return;
            }
        }
        if self.cur.is_none() {
            if !t.starts_with(|c: char| c == '-' || c == '+' || c == '.' || c.is_ascii_digit()) {
                return;
            }
            self.cur = Some(Rec::default());
        }
        self.in_peaks = true;
        if let Some(c) = self.cur.as_mut() {
            c.peaks_text(t);
        }
    }

    // ------------------------------------------------ MGF
    fn mgf(&mut self, l: &str) {
        let t = l.trim();
        if t.is_empty() {
            return;
        }
        let up = t.to_ascii_uppercase();
        if up.starts_with("BEGIN IONS") {
            if self.cur.is_some() {
                self.drop_broken();
            }
            self.cur = Some(Rec::default());
            self.in_peaks = false;
            return;
        }
        if up.starts_with("END IONS") {
            if self.cur.is_some() {
                self.emit();
            }
            return;
        }
        let Some(c) = self.cur.as_mut() else { return };
        if !t.starts_with(|ch: char| ch == '-' || ch == '+' || ch == '.' || ch.is_ascii_digit()) {
            if let Some((k, v)) = split_key(t, '=') {
                let key = key_of(k);
                if key == "pepmass" {
                    // the first number only (the second is the intensity of the precursor)
                    let vt = v.trim();
                    let mut pos = 0;
                    let first = find_number(vt.as_bytes(), &mut pos)
                        .map(|(st, _)| &vt[st..pos])
                        .unwrap_or(vt);
                    c.field("precursormz".into(), first);
                } else {
                    c.field(key, v.trim());
                }
                return;
            }
        }
        self.in_peaks = true;
        c.peaks_text(t);
    }

    // ------------------------------------------------ MassBank record
    fn massbank(&mut self, l: &str) {
        let t = l.trim();
        if t == "//" {
            self.emit();
            return;
        }
        if t.is_empty() {
            return;
        }
        let is_key_line = !l.starts_with([' ', '\t']) && split_key_mb(t).is_some();
        if self.in_peaks && !is_key_line {
            if let Some(c) = self.cur.as_mut() {
                c.peaks_text(t);
            }
            return;
        }
        let Some((k, v)) = split_key_mb(t) else {
            return;
        };
        if k == "ACCESSION"
            && self
                .cur
                .as_ref()
                .is_some_and(|c| c.f.contains_key("accession"))
        {
            self.emit();
        }
        self.in_peaks = false;
        let c = self.cur.get_or_insert_with(Rec::default);
        let v = v.trim();
        // «SUBKEY value» lines
        let sub = |v: &str, name: &str| -> Option<String> {
            let (a, b) = v.split_once(char::is_whitespace)?;
            a.eq_ignore_ascii_case(name).then(|| b.trim().to_string())
        };
        match k {
            "ACCESSION" => c.field("accession".into(), v),
            "RECORD_TITLE" => c.field("recordtitle".into(), v),
            "CH$NAME" => c.field("chname".into(), v),
            "CH$FORMULA" => c.field("formula".into(), v),
            "CH$SMILES" => c.field("smiles".into(), v),
            "CH$LINK" => {
                if let Some(x) = sub(v, "INCHIKEY") {
                    c.field("inchikey".into(), &x);
                }
            }
            "AC$INSTRUMENT" => c.field("instrument".into(), v),
            "AC$INSTRUMENT_TYPE" => c.field("instrumenttype".into(), v),
            "AC$MASS_SPECTROMETRY" => {
                if let Some(x) = sub(v, "ION_MODE") {
                    c.field("ionmode".into(), &x);
                } else if let Some(x) = sub(v, "COLLISION_ENERGY") {
                    c.field("collisionenergy".into(), &x);
                }
            }
            "MS$FOCUSED_ION" => {
                if let Some(x) = sub(v, "PRECURSOR_M/Z") {
                    c.field("precursormz".into(), &x);
                } else if let Some(x) = sub(v, "PRECURSOR_TYPE") {
                    c.field("precursortype".into(), &x);
                }
            }
            "LICENSE" => c.field("license".into(), v),
            "AUTHORS" => c.field("authors".into(), v),
            "PK$PEAK" => self.in_peaks = true,
            _ => {}
        }
    }

    // ------------------------------------------------ JSON (array of objects, or objects one after the other)
    fn push_json(&mut self, data: &[u8]) {
        for &b in data {
            if self.depth == 0 {
                if b == b'{' {
                    self.depth = 1;
                    self.in_str = false;
                    self.esc = false;
                    self.obj.clear();
                    self.obj_skip = false;
                    self.obj.push(b);
                }
                continue;
            }
            if !self.obj_skip {
                if self.obj.len() >= MAX_OBJECT {
                    self.obj_skip = true;
                    self.obj.clear();
                    self.obj.shrink_to_fit();
                    self.stats.dropped += 1;
                    self.stats.broken += 1;
                } else {
                    self.obj.push(b);
                }
            }
            if self.in_str {
                if self.esc {
                    self.esc = false;
                } else if b == b'\\' {
                    self.esc = true;
                } else if b == b'"' {
                    self.in_str = false;
                }
                continue;
            }
            match b {
                b'"' => self.in_str = true,
                b'{' | b'[' => self.depth += 1,
                b'}' | b']' => {
                    self.depth -= 1;
                    if self.depth == 0 {
                        if !self.obj_skip {
                            let o = std::mem::take(&mut self.obj);
                            self.json_object(&o);
                            self.obj = o;
                            self.obj.clear();
                        }
                        self.obj_skip = false;
                    }
                }
                _ => {}
            }
        }
    }

    fn json_object(&mut self, bytes: &[u8]) {
        let v: Value = match serde_json::from_slice(bytes) {
            Ok(v) => v,
            Err(_) => {
                self.stats.dropped += 1;
                self.stats.broken += 1;
                return;
            }
        };
        let mut rec = Rec::default();
        flatten(&v, &mut rec, 0);
        if let Some(s) = finish(rec, &mut self.stats) {
            (self.cb)(s);
        }
    }
}

/// `KEY<sep>value` where KEY is a letter followed by letters, digits, spaces, «_» and «-» (the JS `^([A-Za-z][A-Za-z0-9 _-]*?)\s*:\s*(.*)$`).
fn split_key(t: &str, sep: char) -> Option<(&str, &str)> {
    let i = t.find(sep)?;
    let k = &t[..i];
    let mut ch = k.chars();
    let first = ch.next()?;
    let ok = (first.is_ascii_alphabetic() || (sep == '=' && first == '_'))
        && k.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '_' || c == '-');
    ok.then(|| (k.trim_end(), &t[i + sep.len_utf8()..]))
}

/// MassBank keys («CH$NAME», «AC$MASS_SPECTROMETRY», «LICENSE») end at the first colon.
fn split_key_mb(t: &str) -> Option<(&str, &str)> {
    let i = t.find(':')?;
    let k = &t[..i];
    (!k.is_empty()
        && k.chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '$' || c == '_'))
    .then(|| (k, &t[i + 1..]))
}

fn scalar(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        Value::Bool(b) => Some(b.to_string()),
        Value::Array(a) if !a.is_empty() && a.iter().all(|x| !x.is_object() && !x.is_array()) => {
            scalar(&a[0])
        }
        _ => None,
    }
}

/// Peaks from `[[mz, i], …]`, a JSON string of that, or «mz:i mz:i» / «mz i;mz i» text.
fn peaks_from(v: &Value, rec: &mut Rec) {
    match v {
        Value::Array(a) => {
            for p in a {
                if let Value::Array(pair) = p {
                    if let (Some(m), Some(i)) =
                        (pair.first().and_then(num_of), pair.get(1).and_then(num_of))
                    {
                        rec.peak(m, i);
                    }
                } else if let Value::Object(o) = p {
                    let m = o.get("mz").or_else(|| o.get("m/z")).and_then(num_of);
                    let i = o.get("intensity").or_else(|| o.get("i")).and_then(num_of);
                    if let (Some(m), Some(i)) = (m, i) {
                        rec.peak(m, i);
                    }
                }
            }
        }
        Value::String(s) => {
            let t = s.trim();
            if t.starts_with('[') {
                if let Ok(j) = serde_json::from_str::<Value>(t) {
                    if j.is_array() {
                        peaks_from(&j, rec);
                    }
                }
            } else {
                for seg in t.split([' ', ';', '\n', '\t']).filter(|x| !x.is_empty()) {
                    let b = seg.as_bytes();
                    let mut pos = 0;
                    if let (Some(m), Some(i)) = (next_number(b, &mut pos), next_number(b, &mut pos))
                    {
                        rec.peak(m, i);
                    }
                }
            }
        }
        _ => {}
    }
}

fn num_of(v: &Value) -> Option<f64> {
    match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse().ok(),
        _ => None,
    }
}

/// Fills the record: scalar members of this level first, then `metaData` name/value lists, then the nested objects (MoNA «compound»).
fn flatten(v: &Value, rec: &mut Rec, depth: u32) {
    let Value::Object(o) = v else { return };
    if depth > 4 {
        return;
    }
    for (k, val) in o {
        let key = key_of(k);
        if matches!(key.as_str(), "peaks" | "peaksjson" | "spectrum") {
            if rec.mz.is_empty() {
                peaks_from(val, rec);
            }
        } else if let Some(s) = scalar(val) {
            rec.field(key, &s);
        }
    }
    for (k, val) in o {
        let key = key_of(k);
        if let Value::Array(a) = val {
            if key == "names" {
                if let Some(n) = a.iter().find_map(|x| x.get("name").and_then(scalar)) {
                    rec.field("name".into(), &n);
                }
            }
            for item in a {
                if let Value::Object(m) = item {
                    if let (Some(n), Some(val)) = (
                        m.get("name").and_then(scalar),
                        m.get("value").and_then(scalar),
                    ) {
                        if m.len() <= 6 && m.contains_key("value") {
                            rec.field(key_of(&n), &val);
                        }
                    }
                }
            }
        }
    }
    for (k, val) in o {
        match val {
            Value::Object(_) => flatten(val, rec, depth + 1),
            Value::Array(a)
                if matches!(
                    key_of(k).as_str(),
                    "compound" | "library" | "submitter" | "meta"
                ) =>
            {
                for x in a {
                    flatten(x, rec, depth + 1);
                }
            }
            _ => {}
        }
    }
}

/// Reads a whole text in memory (tests, small files): all the spectra and the statistics.
pub fn parse_all(fmt: Format, data: &[u8]) -> (Vec<Spectrum>, Stats) {
    let mut out = Vec::new();
    let mut p = Parser::new(fmt, |s| out.push(s));
    p.push(data);
    p.end();
    let st = p.stats;
    (out, st)
}
