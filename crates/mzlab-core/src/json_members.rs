//! Reading a few top-level members of a large JSON text: the other values are checked exactly as `json::parse` checks them, but never built.
//! `corsa/1` reads two or three keys of every output file of a run; a group step's `ionnet/1` can be hundreds of MB.

use crate::error::Result;
use crate::json::{bad, Value, P};

impl P<'_> {
    /// Checks a string as `string` does, in the scratch buffer.
    fn skip_string(&mut self) -> Result<()> {
        let mut out = std::mem::take(&mut self.scratch);
        out.clear();
        let r = self.string_into(&mut out);
        let ok = r.is_ok() && std::str::from_utf8(&out).is_ok();
        self.scratch = out;
        r.and_then(|()| if ok { Ok(()) } else { Err(bad()) })
    }
    /// Checks one value as `value` does, building nothing that outlives the call.
    fn skip(&mut self, depth: usize) -> Result<()> {
        if depth > 64 {
            return Err(bad());
        }
        self.ws();
        let (open, close) = match self.b.get(self.i).copied() {
            Some(b'{') => (b'{', b'}'),
            Some(b'[') => (b'[', b']'),
            Some(b'"') => return self.skip_string(),
            Some(b't') => return self.lit(b"true", Value::Null).map(drop),
            Some(b'f') => return self.lit(b"false", Value::Null).map(drop),
            Some(b'n') => return self.lit(b"null", Value::Null).map(drop),
            Some(b'N') => return self.lit(b"NaN", Value::Null).map(drop),
            Some(_) => return self.number().map(drop),
            None => return Err(bad()),
        };
        self.i += 1;
        self.ws();
        if self.b.get(self.i) == Some(&close) {
            self.i += 1;
            return Ok(());
        }
        loop {
            if open == b'{' {
                self.ws();
                self.skip_string()?;
                self.eat(b':')?;
            }
            self.skip(depth + 1)?;
            self.ws();
            match self.b.get(self.i) {
                Some(b',') => self.i += 1,
                Some(&c) if c == close => {
                    self.i += 1;
                    return Ok(());
                }
                _ => return Err(bad()),
            }
        }
    }
}

/// The top-level object of `b` with only the members named in `keep` (in file order, repeats kept): the other values are checked as `parse` checks
/// them but never built, so a large output file read for a few keys costs no tree. Fails where `parse` fails; a top level that is not an object gives `Null`.
pub fn parse_members(b: &[u8], keep: &[&str]) -> Result<Value> {
    let mut p = P {
        b,
        i: 0,
        scratch: Vec::new(),
    };
    p.ws();
    let v = if p.b.get(p.i) == Some(&b'{') {
        p.i += 1;
        let mut o = Vec::new();
        p.ws();
        if p.b.get(p.i) == Some(&b'}') {
            p.i += 1;
        } else {
            loop {
                p.ws();
                let k = p.string()?;
                p.eat(b':')?;
                if keep.contains(&k.as_str()) {
                    o.push((k, p.value(1)?));
                } else {
                    p.skip(1)?;
                }
                p.ws();
                match p.b.get(p.i) {
                    Some(b',') => p.i += 1,
                    Some(b'}') => {
                        p.i += 1;
                        break;
                    }
                    _ => return Err(bad()),
                }
            }
        }
        Value::Obj(o)
    } else {
        p.skip(0)?;
        Value::Null
    };
    p.ws();
    if p.i == b.len() {
        Ok(v)
    } else {
        Err(bad())
    }
}

#[cfg(test)]
#[path = "json_members_tests.rs"]
mod tests;
