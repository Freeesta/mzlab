// Ported from Attributio campaign.py.
//! The part of TOML that a `campaign.toml` needs: tables (`[a]`, `[a.b]`, `[sites."Site A"]`), dotted and quoted keys, strings (basic with escapes, literal),
//! integers, floats, booleans, arrays (also over several lines, with comments and a trailing comma) and inline tables on one line. Anything else (arrays of
//! tables, multi-line strings, dates, hexadecimal numbers) is an error that names the line.

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Array(Vec<Value>),
    Table(Table),
}

/// A table keeps the order of the file (the order of the sites matters).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Table(pub Vec<(String, Value)>);

#[derive(Clone, Debug, PartialEq)]
pub struct TomlError {
    pub line: usize,
    pub msg: String,
}

impl Table {
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v)
    }
    pub fn table(&self, key: &str) -> Option<&Table> {
        match self.get(key) {
            Some(Value::Table(t)) => Some(t),
            _ => None,
        }
    }
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.0.iter().map(|(k, _)| k.as_str())
    }
    fn slot(&mut self, key: &str) -> Option<&mut Value> {
        self.0.iter_mut().find(|(k, _)| k == key).map(|(_, v)| v)
    }
}

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }
    /// An integer or a float as a float (`float(x)`).
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Int(i) => Some(*i as f64),
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }
}

struct P<'a> {
    b: &'a [u8],
    s: &'a str,
    i: usize,
    line: usize,
}

type R<T> = std::result::Result<T, TomlError>;

pub fn parse(text: &str) -> R<Table> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut p = P {
        b: text.as_bytes(),
        s: text,
        i: 0,
        line: 1,
    };
    let mut root = Table::default();
    let mut cur: Vec<String> = Vec::new();
    let mut defined: Vec<Vec<String>> = Vec::new();
    loop {
        p.skip_all();
        match p.peek() {
            None => return Ok(root),
            Some(b'[') => {
                if p.b.get(p.i + 1) == Some(&b'[') {
                    return Err(p.err("arrays of tables ([[...]]) are not supported"));
                }
                p.i += 1;
                cur = p.key_path(b']')?;
                p.expect(b']')?;
                p.end_of_line()?;
                if defined.contains(&cur) {
                    return Err(p.err(&format!("table [{}] is defined twice", cur.join("."))));
                }
                defined.push(cur.clone());
                open_table(&mut root, &cur).map_err(|m| p.err(&m))?;
            }
            Some(_) => {
                let keys = p.key_path(b'=')?;
                p.expect(b'=')?;
                p.skip_ws();
                let v = p.value()?;
                p.end_of_line()?;
                let mut path = cur.clone();
                path.extend(keys);
                insert(&mut root, &path, v).map_err(|m| p.err(&m))?;
            }
        }
    }
}

/// The table at `path`, created (with the parents) when it is not there.
fn open_table<'t>(
    root: &'t mut Table,
    path: &[String],
) -> std::result::Result<&'t mut Table, String> {
    let mut t = root;
    for k in path {
        if t.slot(k).is_none() {
            t.0.push((k.clone(), Value::Table(Table::default())));
        }
        t = match t.slot(k) {
            Some(Value::Table(x)) => x,
            _ => return Err(format!("key '{k}' is not a table")),
        };
    }
    Ok(t)
}

fn insert(root: &mut Table, path: &[String], v: Value) -> std::result::Result<(), String> {
    let (last, parents) = path.split_last().ok_or("empty key")?;
    let t = open_table(root, parents)?;
    if t.slot(last).is_some() {
        return Err(format!("key '{last}' is defined twice"));
    }
    t.0.push((last.clone(), v));
    Ok(())
}

impl P<'_> {
    fn err(&self, m: &str) -> TomlError {
        TomlError {
            line: self.line,
            msg: m.to_string(),
        }
    }
    fn peek(&self) -> Option<u8> {
        self.b.get(self.i).copied()
    }
    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t')) {
            self.i += 1;
        }
    }
    fn skip_comment(&mut self) {
        if self.peek() == Some(b'#') {
            while !matches!(self.peek(), None | Some(b'\n')) {
                self.i += 1;
            }
        }
    }
    /// White space, comments and line ends.
    fn skip_all(&mut self) {
        loop {
            self.skip_ws();
            self.skip_comment();
            match self.peek() {
                Some(b'\n') => {
                    self.i += 1;
                    self.line += 1;
                }
                Some(b'\r') => self.i += 1,
                _ => return,
            }
        }
    }
    fn end_of_line(&mut self) -> R<()> {
        self.skip_ws();
        self.skip_comment();
        match self.peek() {
            None | Some(b'\n') | Some(b'\r') => Ok(()),
            Some(_) => Err(self.err("unexpected text after the value")),
        }
    }
    fn expect(&mut self, c: u8) -> R<()> {
        self.skip_ws();
        if self.peek() == Some(c) {
            self.i += 1;
            Ok(())
        } else {
            Err(self.err(&format!("expected '{}'", c as char)))
        }
    }
    /// A dotted key: bare words (`A-Za-z0-9_-`) or quoted strings, up to `stop`.
    fn key_path(&mut self, stop: u8) -> R<Vec<String>> {
        let mut out = Vec::new();
        loop {
            self.skip_ws();
            match self.peek() {
                Some(b'"') => out.push(self.basic_string()?),
                Some(b'\'') => out.push(self.literal_string()?),
                Some(c) if c.is_ascii_alphanumeric() || c == b'_' || c == b'-' => {
                    let s = self.i;
                    while matches!(self.peek(), Some(c) if c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
                    {
                        self.i += 1;
                    }
                    out.push(self.s[s..self.i].to_string());
                }
                _ => return Err(self.err("expected a key")),
            }
            self.skip_ws();
            match self.peek() {
                Some(b'.') => self.i += 1,
                Some(c) if c == stop => return Ok(out),
                _ => return Err(self.err("expected '.' or the end of the key")),
            }
        }
    }
    fn basic_string(&mut self) -> R<String> {
        if self.s[self.i..].starts_with("\"\"\"") {
            return Err(self.err("multi-line strings are not supported"));
        }
        self.i += 1;
        let mut out = String::new();
        loop {
            let rest = &self.s[self.i..];
            let c = rest
                .chars()
                .next()
                .ok_or_else(|| self.err("unterminated string"))?;
            self.i += c.len_utf8();
            match c {
                '"' => return Ok(out),
                '\n' => return Err(self.err("unterminated string")),
                '\\' => out.push(self.escape()?),
                c => out.push(c),
            }
        }
    }
    fn escape(&mut self) -> R<char> {
        let c = self.s[self.i..]
            .chars()
            .next()
            .ok_or_else(|| self.err("bad escape"))?;
        self.i += c.len_utf8();
        let hex = |p: &mut Self, n: usize| -> R<char> {
            let h = p.s.get(p.i..p.i + n).ok_or_else(|| p.err("bad escape"))?;
            p.i += n;
            u32::from_str_radix(h, 16)
                .ok()
                .and_then(char::from_u32)
                .ok_or_else(|| p.err("bad escape"))
        };
        Ok(match c {
            'n' => '\n',
            't' => '\t',
            'r' => '\r',
            'b' => '\u{8}',
            'f' => '\u{c}',
            '"' => '"',
            '\\' => '\\',
            'u' => hex(self, 4)?,
            'U' => hex(self, 8)?,
            _ => return Err(self.err("bad escape")),
        })
    }
    fn literal_string(&mut self) -> R<String> {
        if self.s[self.i..].starts_with("'''") {
            return Err(self.err("multi-line strings are not supported"));
        }
        self.i += 1;
        let s = self.i;
        while !matches!(self.peek(), Some(b'\'') | Some(b'\n') | None) {
            self.i += 1;
        }
        if self.peek() != Some(b'\'') {
            return Err(self.err("unterminated string"));
        }
        self.i += 1;
        Ok(self.s[s..self.i - 1].to_string())
    }
    fn value(&mut self) -> R<Value> {
        match self.peek() {
            Some(b'"') => Ok(Value::Str(self.basic_string()?)),
            Some(b'\'') => Ok(Value::Str(self.literal_string()?)),
            Some(b'[') => self.array(),
            Some(b'{') => self.inline_table(),
            Some(_) => self.scalar(),
            None => Err(self.err("expected a value")),
        }
    }
    fn array(&mut self) -> R<Value> {
        self.i += 1;
        let mut items = Vec::new();
        loop {
            self.skip_all();
            if self.peek() == Some(b']') {
                self.i += 1;
                return Ok(Value::Array(items));
            }
            items.push(self.value()?);
            self.skip_all();
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b']') => {}
                _ => return Err(self.err("expected ',' or ']' in the array")),
            }
        }
    }
    fn inline_table(&mut self) -> R<Value> {
        self.i += 1;
        let mut t = Table::default();
        self.skip_ws();
        if self.peek() == Some(b'}') {
            self.i += 1;
            return Ok(Value::Table(t));
        }
        loop {
            let keys = self.key_path(b'=')?;
            self.expect(b'=')?;
            self.skip_ws();
            let v = self.value()?;
            insert(&mut t, &keys, v).map_err(|m| self.err(&m))?;
            self.skip_ws();
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b'}') => {
                    self.i += 1;
                    return Ok(Value::Table(t));
                }
                _ => return Err(self.err("expected ',' or '}' in the inline table")),
            }
        }
    }
    fn scalar(&mut self) -> R<Value> {
        let s = self.i;
        while matches!(self.peek(), Some(c) if !matches!(c, b',' | b']' | b'}' | b'#' | b' ' | b'\t' | b'\n' | b'\r'))
        {
            self.i += 1;
        }
        let w = &self.s[s..self.i];
        match w {
            "true" => return Ok(Value::Bool(true)),
            "false" => return Ok(Value::Bool(false)),
            _ => {}
        }
        let clean = w.replace('_', "");
        let digits = clean.trim_start_matches(['+', '-']);
        if !digits.starts_with(|c: char| c.is_ascii_digit()) || w.contains(':') {
            return Err(self.err(&format!("unsupported value '{w}'")));
        }
        if let Ok(i) = clean.parse::<i64>() {
            return Ok(Value::Int(i));
        }
        let float_like = clean
            .bytes()
            .all(|c| c.is_ascii_digit() || matches!(c, b'+' | b'-' | b'.' | b'e' | b'E'));
        match clean.parse::<f64>() {
            Ok(f) if float_like => Ok(Value::Float(f)),
            _ => Err(self.err(&format!(
                "unsupported value '{w}' (dates, hexadecimal and similar are not supported)"
            ))),
        }
    }
}
