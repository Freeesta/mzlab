//! Streaming readers of spectral libraries: MSP, MGF, MassBank records (`.txt`), JSON arrays (MoNA, GNPS) and JSON lines (mzMine).
//! `Parser::push` takes the file in any chunking (even in the middle of a character) and calls the callback for every usable record.
//! A record without a precursor or without peaks is counted and dropped, never an error; text fields are cut at `MAX_FIELD_BYTES`.

use crate::error::{Error, Result};
use crate::spec::{Meta, Spectrum, MAX_FIELD_BYTES, MAX_RAW_PEAKS};
use serde_json::Value;

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
    let h = String::from_utf8_lossy(head.get(..8192).unwrap_or(head));
    let first = h.trim_start_matches('\u{feff}').trim_start();
    if h.as_bytes()
        .windows(10)
        .any(|w| w.eq_ignore_ascii_case(b"BEGIN IONS"))
    {
        Ok(Format::Mgf)
    } else if first.starts_with('{') || first.starts_with('[') {
        Ok(Format::Json)
    } else if h
        .lines()
        .any(|l| starts_with_ci(l.trim_start(), "ACCESSION:"))
    {
        Ok(Format::MassBank)
    } else if h.lines().any(|l| {
        let l = l.trim();
        starts_with_ci(l, "name:") || starts_with_ci(l, "name :")
    }) {
        Ok(Format::Msp)
    } else {
        Err(Error::new("lib.err.unknown"))
    }
}

/// `t` starts with `p`, ignoring the case of ASCII letters (`p` is ASCII).
fn starts_with_ci(t: &str, p: &str) -> bool {
    t.as_bytes()
        .get(..p.len())
        .is_some_and(|h| h.eq_ignore_ascii_case(p.as_bytes()))
}

// ---------------------------------------------------------------- numbers

/// The next number (the JS regex `[-+]?(\d+\.?\d*|\.\d+)([eE][-+]?\d+)?`) at or after `*pos`.
fn next_number(s: &[u8], pos: &mut usize) -> Option<f64> {
    find_number(s, pos).map(|(_, v)| v)
}

/// Like `next_number`, also giving the start of the match.
fn find_number(s: &[u8], pos: &mut usize) -> Option<(usize, f64)> {
    find_number_c(s, pos, false)
}

/// `find_number`; with `comma` a «,» counts as the decimal point (the text is read as if every comma were a point).
fn find_number_c(s: &[u8], pos: &mut usize, comma: bool) -> Option<(usize, f64)> {
    // Out-of-range reads give 0, which matches none of the characters tested below.
    let at = |k: usize| match s.get(k).copied().unwrap_or(0) {
        b',' if comma => b'.',
        c => c,
    };
    let n = s.len();
    let mut i = *pos;
    while i < n {
        let start = i;
        let mut j = i;
        if at(j) == b'+' || at(j) == b'-' {
            j += 1;
        }
        let d0 = j;
        while at(j).is_ascii_digit() {
            j += 1;
        }
        let mut ok = j > d0;
        if ok {
            if at(j) == b'.' {
                j += 1;
                while at(j).is_ascii_digit() {
                    j += 1;
                }
            }
        } else if at(j) == b'.' && at(j + 1).is_ascii_digit() {
            j += 1;
            while at(j).is_ascii_digit() {
                j += 1;
            }
            ok = true;
        }
        if ok {
            if at(j) == b'e' || at(j) == b'E' {
                let mut k = j + 1;
                if at(k) == b'+' || at(k) == b'-' {
                    k += 1;
                }
                let e0 = k;
                while at(k).is_ascii_digit() {
                    k += 1;
                }
                if k > e0 {
                    j = k;
                }
            }
            *pos = j;
            let text = s.get(start..j).and_then(|b| std::str::from_utf8(b).ok());
            return match text {
                Some(t) if comma && t.contains(',') => t.replace(',', ".").parse().ok(),
                Some(t) => t.parse().ok(),
                None => None,
            }
            .map(|v| (start, v));
        }
        i += 1;
    }
    None
}

/// JS `parseFloat` after turning a comma into a point: a numeric prefix, else NaN.
fn parse_float(s: &str) -> f64 {
    let b = s.trim_start().as_bytes();
    let mut pos = 0;
    match find_number_c(b, &mut pos, true) {
        Some((_, v))
            if b.first()
                .is_some_and(|c| matches!(c, b'+' | b'-' | b'.' | b',' | b'0'..=b'9')) =>
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

mod rec;
use rec::{finish, norm_key, Rec, K};

// ---------------------------------------------------------------- the parser

pub struct Parser<F: FnMut(Spectrum)> {
    fmt: Format,
    cb: F,
    pub stats: Stats,
    line: Vec<u8>,
    skipping: bool,
    cur: Option<Rec>,
    /// The record of the last spectrum, emptied, for the next one.
    spare: Option<Rec>,
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
            spare: None,
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
                self.add_line_bytes(data.get(start..i).unwrap_or_default());
                self.end_line();
                start = i + 1;
            }
        }
        self.add_line_bytes(data.get(start..).unwrap_or_default());
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
        if let Some(mut c) = self.cur.take() {
            let s = finish(&mut c, &mut self.stats);
            c.reset();
            self.spare = Some(c);
            if let Some(s) = s {
                (self.cb)(s);
            }
        }
    }

    fn new_rec(&mut self) -> Rec {
        self.spare.take().unwrap_or_default()
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
            let mut kb = [0u8; 32];
            let key = norm_key(k, &mut kb);
            if !self.in_peaks || key == "name" {
                // a «Name:» line always starts a new record, even without the blank line
                if self.cur.is_some()
                    && key == "name"
                    && (self.in_peaks || self.cur.as_ref().is_some_and(|c| c.has(K::Name)))
                {
                    self.emit();
                }
                if self.cur.is_none() {
                    self.cur = Some(self.new_rec());
                }
                if key == "numpeaks" {
                    self.in_peaks = true;
                    return;
                }
                if let Some(c) = self.cur.as_mut() {
                    c.field_str(key, v.trim());
                }
                return;
            }
        }
        if self.cur.is_none() {
            if !t.starts_with(|c: char| c == '-' || c == '+' || c == '.' || c.is_ascii_digit()) {
                return;
            }
            self.cur = Some(self.new_rec());
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
        if starts_with_ci(t, "BEGIN IONS") {
            if self.cur.is_some() {
                self.drop_broken();
            }
            self.cur = Some(self.new_rec());
            self.in_peaks = false;
            return;
        }
        if starts_with_ci(t, "END IONS") {
            if self.cur.is_some() {
                self.emit();
            }
            return;
        }
        let Some(c) = self.cur.as_mut() else { return };
        if !t.starts_with(|ch: char| ch == '-' || ch == '+' || ch == '.' || ch.is_ascii_digit())
            && let Some((k, v)) = split_key(t, '=')
        {
            let mut kb = [0u8; 32];
            let key = norm_key(k, &mut kb);
            if key == "pepmass" {
                // the first number only (the second is the intensity of the precursor)
                let vt = v.trim();
                let mut pos = 0;
                let first = find_number(vt.as_bytes(), &mut pos)
                    .map(|(st, _)| &vt[st..pos])
                    .unwrap_or(vt);
                c.field(K::PrecursorMz, first);
            } else {
                c.field_str(key, v.trim());
            }
            return;
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
        if k == "ACCESSION" && self.cur.as_ref().is_some_and(|c| c.has(K::Accession)) {
            self.emit();
        }
        self.in_peaks = false;
        if self.cur.is_none() {
            self.cur = Some(self.new_rec());
        }
        let Some(c) = self.cur.as_mut() else { return };
        let v = v.trim();
        // «SUBKEY value» lines
        let sub = |v: &str, name: &str| -> Option<String> {
            let (a, b) = v.split_once(char::is_whitespace)?;
            a.eq_ignore_ascii_case(name).then(|| b.trim().to_string())
        };
        match k {
            "ACCESSION" => c.field(K::Accession, v),
            "RECORD_TITLE" => c.field(K::RecordTitle, v),
            "CH$NAME" => c.field(K::ChName, v),
            "CH$FORMULA" => c.field(K::Formula, v),
            "CH$SMILES" => c.field(K::Smiles, v),
            "CH$LINK" => {
                if let Some(x) = sub(v, "INCHIKEY") {
                    c.field(K::InchiKey, &x);
                }
            }
            "AC$INSTRUMENT" => c.field(K::Instrument, v),
            "AC$INSTRUMENT_TYPE" => c.field(K::InstrumentType, v),
            "AC$CHROMATOGRAPHY" => {
                if let Some(x) = sub(v, "RETENTION_TIME") {
                    c.field(K::RetentionTime, &x);
                } else if let Some(x) = sub(v, "KOVATS_RI") {
                    c.field(K::KovatsRi, &x);
                }
            }
            "AC$MASS_SPECTROMETRY" => {
                if let Some(x) = sub(v, "ION_MODE") {
                    c.field(K::IonMode, &x);
                } else if let Some(x) = sub(v, "IONIZATION") {
                    c.field(K::Ionization, &x);
                } else if let Some(x) = sub(v, "COLLISION_ENERGY") {
                    c.field(K::CollisionEnergy, &x);
                }
            }
            "MS$FOCUSED_ION" => {
                if let Some(x) = sub(v, "PRECURSOR_M/Z") {
                    c.field(K::PrecursorMz, &x);
                } else if let Some(x) = sub(v, "PRECURSOR_TYPE") {
                    c.field(K::PrecursorType, &x);
                }
            }
            "LICENSE" => c.field(K::License, v),
            "AUTHORS" => c.field(K::Authors, v),
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
        let mut rec = self.new_rec();
        flatten(&v, &mut rec, 0);
        let s = finish(&mut rec, &mut self.stats);
        rec.reset();
        self.spare = Some(rec);
        if let Some(s) = s {
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
        && k.chars().all(|c| {
            c.is_ascii_alphanumeric()
                || c == ' '
                || c == '_'
                || c == '-'
                || (sep == ':' && c == '#')
        });
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
        Value::Array(a) if a.iter().all(|x| !x.is_object() && !x.is_array()) => {
            a.first().and_then(scalar)
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
                if let Ok(j) = serde_json::from_str::<Value>(t)
                    && j.is_array()
                {
                    peaks_from(&j, rec);
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
            rec.field_str(&key, &s);
        }
    }
    for (k, val) in o {
        let key = key_of(k);
        if let Value::Array(a) = val {
            if key == "names"
                && let Some(n) = a.iter().find_map(|x| x.get("name").and_then(scalar))
            {
                rec.field(K::Name, &n);
            }
            for item in a {
                if let Value::Object(m) = item
                    && let (Some(n), Some(val)) = (
                        m.get("name").and_then(scalar),
                        m.get("value").and_then(scalar),
                    )
                    && m.len() <= 6
                    && m.contains_key("value")
                {
                    rec.field_str(&key_of(&n), &val);
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
