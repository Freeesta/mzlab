//! A small JSON reader and writer (the `.mzidx` metadata and scan table are JSON; pulling in serde would cost size in the WASM).

use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Value>),
    Obj(Vec<(String, Value)>),
}

impl Value {
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Obj(o) => o.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    pub fn at(&self, i: usize) -> Option<&Value> {
        match self {
            Value::Arr(a) => a.get(i),
            _ => None,
        }
    }
    pub fn f64(&self) -> Option<f64> {
        match self {
            Value::Num(n) => Some(*n),
            _ => None,
        }
    }
    pub fn str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }
    pub fn arr(&self) -> &[Value] {
        match self {
            Value::Arr(a) => a,
            _ => &[],
        }
    }
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }
}

fn bad() -> Error {
    Error::new("error.badIndex")
}

struct P<'a> {
    b: &'a [u8],
    i: usize,
}

impl P<'_> {
    fn ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\n' | b'\r' | b'\t') {
            self.i += 1;
        }
    }
    fn eat(&mut self, c: u8) -> Result<()> {
        self.ws();
        if self.b.get(self.i) == Some(&c) {
            self.i += 1;
            Ok(())
        } else {
            Err(bad())
        }
    }
    fn lit(&mut self, s: &[u8], v: Value) -> Result<Value> {
        if self.b[self.i..].starts_with(s) {
            self.i += s.len();
            Ok(v)
        } else {
            Err(bad())
        }
    }
    fn value(&mut self, depth: usize) -> Result<Value> {
        if depth > 64 {
            return Err(bad());
        }
        self.ws();
        match self.b.get(self.i).copied() {
            Some(b'{') => {
                self.i += 1;
                let mut o = Vec::new();
                self.ws();
                if self.b.get(self.i) == Some(&b'}') {
                    self.i += 1;
                    return Ok(Value::Obj(o));
                }
                loop {
                    self.ws();
                    let k = self.string()?;
                    self.eat(b':')?;
                    o.push((k, self.value(depth + 1)?));
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b'}') => {
                            self.i += 1;
                            return Ok(Value::Obj(o));
                        }
                        _ => return Err(bad()),
                    }
                }
            }
            Some(b'[') => {
                self.i += 1;
                let mut a = Vec::new();
                self.ws();
                if self.b.get(self.i) == Some(&b']') {
                    self.i += 1;
                    return Ok(Value::Arr(a));
                }
                loop {
                    a.push(self.value(depth + 1)?);
                    self.ws();
                    match self.b.get(self.i) {
                        Some(b',') => self.i += 1,
                        Some(b']') => {
                            self.i += 1;
                            return Ok(Value::Arr(a));
                        }
                        _ => return Err(bad()),
                    }
                }
            }
            Some(b'"') => Ok(Value::Str(self.string()?)),
            Some(b't') => self.lit(b"true", Value::Bool(true)),
            Some(b'f') => self.lit(b"false", Value::Bool(false)),
            Some(b'n') => self.lit(b"null", Value::Null),
            Some(b'N') => self.lit(b"NaN", Value::Num(f64::NAN)),
            Some(_) => self.number(),
            None => Err(bad()),
        }
    }
    fn number(&mut self) -> Result<Value> {
        let s = self.i;
        while self.i < self.b.len()
            && matches!(
                self.b[self.i],
                b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E'
            )
        {
            self.i += 1;
        }
        std::str::from_utf8(&self.b[s..self.i])
            .ok()
            .and_then(|t| t.parse::<f64>().ok())
            .map(Value::Num)
            .ok_or_else(bad)
    }
    fn string(&mut self) -> Result<String> {
        if self.b.get(self.i) != Some(&b'"') {
            return Err(bad());
        }
        self.i += 1;
        let mut out: Vec<u8> = Vec::new();
        loop {
            let c = *self.b.get(self.i).ok_or_else(bad)?;
            self.i += 1;
            match c {
                b'"' => return String::from_utf8(out).map_err(|_| bad()),
                b'\\' => {
                    let e = *self.b.get(self.i).ok_or_else(bad)?;
                    self.i += 1;
                    match e {
                        b'n' => out.push(b'\n'),
                        b't' => out.push(b'\t'),
                        b'r' => out.push(b'\r'),
                        b'b' => out.push(8),
                        b'f' => out.push(12),
                        b'u' => {
                            let mut cp = self.hex4()?;
                            if (0xD800..0xDC00).contains(&cp)
                                && self.b.get(self.i) == Some(&b'\\')
                                && self.b.get(self.i + 1) == Some(&b'u')
                            {
                                self.i += 2;
                                let lo = self.hex4()?;
                                cp = 0x10000
                                    + ((cp - 0xD800) << 10)
                                    + (lo.wrapping_sub(0xDC00) & 0x3FF);
                            }
                            let ch = char::from_u32(cp).unwrap_or('\u{FFFD}');
                            let mut tmp = [0u8; 4];
                            out.extend_from_slice(ch.encode_utf8(&mut tmp).as_bytes());
                        }
                        other => out.push(other),
                    }
                }
                _ => out.push(c),
            }
        }
    }
    fn hex4(&mut self) -> Result<u32> {
        let h = self.b.get(self.i..self.i + 4).ok_or_else(bad)?;
        self.i += 4;
        u32::from_str_radix(std::str::from_utf8(h).map_err(|_| bad())?, 16).map_err(|_| bad())
    }
}

pub fn parse(b: &[u8]) -> Result<Value> {
    let mut p = P { b, i: 0 };
    let v = p.value(0)?;
    p.ws();
    if p.i == b.len() {
        Ok(v)
    } else {
        Err(bad())
    }
}

/// JSON text builder: the routes write their answers straight into it.
#[derive(Default)]
pub struct Out(pub String);

impl Out {
    pub fn raw(&mut self, s: &str) -> &mut Self {
        self.0.push_str(s);
        self
    }
    pub fn str(&mut self, s: &str) -> &mut Self {
        self.0.push('"');
        for c in s.chars() {
            match c {
                '"' => self.0.push_str("\\\""),
                '\\' => self.0.push_str("\\\\"),
                '\n' => self.0.push_str("\\n"),
                '\r' => self.0.push_str("\\r"),
                '\t' => self.0.push_str("\\t"),
                c if (c as u32) < 0x20 => self.0.push_str(&format!("\\u{:04x}", c as u32)),
                c => self.0.push(c),
            }
        }
        self.0.push('"');
        self
    }
    /// A finite number; NaN and infinities become `null` (never valid JSON otherwise).
    pub fn num(&mut self, v: f64) -> &mut Self {
        if v.is_finite() {
            self.0.push_str(&format!("{v}"));
        } else {
            self.0.push_str("null");
        }
        self
    }
    pub fn int(&mut self, v: i64) -> &mut Self {
        self.0.push_str(&v.to_string());
        self
    }
    pub fn opt(&mut self, v: Option<f64>) -> &mut Self {
        match v {
            Some(x) => self.num(x),
            None => self.raw("null"),
        }
    }
    /// A list of numbers as they are.
    pub fn floats(&mut self, a: &[f64]) -> &mut Self {
        self.0.push('[');
        for (i, &v) in a.iter().enumerate() {
            if i > 0 {
                self.0.push(',');
            }
            self.num(v);
        }
        self.0.push(']');
        self
    }
    /// A list of numbers, each rounded to `nd` decimals like Python's `round(x, nd)` (the correctly rounded decimal of the exact double).
    pub fn rounded(&mut self, a: &[f64], nd: usize) -> &mut Self {
        self.0.push('[');
        for (i, &v) in a.iter().enumerate() {
            if i > 0 {
                self.0.push(',');
            }
            self.num(round(v, nd));
        }
        self.0.push(']');
        self
    }
}

/// Python's `round(v, nd)`: the double nearest to the correctly rounded decimal of the exact binary value.
pub fn round(v: f64, nd: usize) -> f64 {
    if !v.is_finite() {
        return v;
    }
    format!("{v:.nd$}").parse().unwrap_or(v)
}

/// Python's `format(x, "g")` (six significant digits, no trailing zeros).
pub fn fmt_g(x: f64) -> String {
    if x == 0.0 {
        return "0".into();
    }
    let e = format!("{x:.5e}");
    let (mant, exp) = e.split_once('e').unwrap_or((&e, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    if !(-4..6).contains(&exp) {
        let m = mant.trim_end_matches('0').trim_end_matches('.');
        return format!("{m}e{}{:02}", if exp < 0 { '-' } else { '+' }, exp.abs());
    }
    let dec = (5 - exp).max(0) as usize;
    let s = format!("{x:.dec$}");
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_writes() {
        let v = parse(r#"{"a":[1,2.5,null,"xè\n"],"b":{"c":true}}"#.as_bytes()).unwrap();
        assert_eq!(v.get("a").unwrap().at(1).unwrap().f64(), Some(2.5));
        assert_eq!(v.get("a").unwrap().at(3).unwrap().str(), Some("x\u{e8}\n"));
        assert_eq!(v.get("b").unwrap().get("c").unwrap().bool(), Some(true));
        assert!(parse(b"{").is_err());
    }

    #[test]
    fn format_g_like_python() {
        assert_eq!(fmt_g(25.0), "25");
        assert_eq!(fmt_g(25.5), "25.5");
        assert_eq!(fmt_g(0.0001), "0.0001");
        assert_eq!(fmt_g(1234567.0), "1.23457e+06");
        assert_eq!(fmt_g(0.00001234), "1.234e-05");
        assert_eq!(fmt_g(100.0), "100");
    }

    #[test]
    fn rounding_like_python() {
        assert_eq!(round(2.675, 2), 2.67);
        assert_eq!(round(0.5, 0), 0.0);
        assert_eq!(round(1.5, 0), 2.0);
    }
}
