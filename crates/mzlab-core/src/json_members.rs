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
mod tests {
    use super::*;
    use crate::json::parse;

    /// `parse_members` keeps the members `parse` gives for the same keys and fails on the same texts.
    #[test]
    fn members_match_the_whole_parse() {
        let keep = ["format", "rows"];
        let ok = [
            r#" {"format":"annot/1","edges":[{"a":1,"b":[true,false,null,NaN,-1.5e3]}],"rows":[{"x":"è😀\n"}],"rows":2} "#,
            r#"{"edges":{},"other":[],"format":"x"}"#,
            "{}",
        ];
        for t in ok {
            let (whole, some) = (
                parse(t.as_bytes()).ok(),
                parse_members(t.as_bytes(), &keep).ok(),
            );
            let (Some(Value::Obj(w)), Some(Value::Obj(s))) = (whole, some) else {
                panic!("{t}");
            };
            let kept: Vec<_> = w
                .into_iter()
                .filter(|(k, _)| keep.contains(&k.as_str()))
                .collect();
            assert_eq!(format!("{kept:?}"), format!("{s:?}"), "{t}");
        }
        assert!(matches!(
            parse_members(b"[1,{\"a\":2}]", &keep),
            Ok(Value::Null)
        ));
        let deep = format!("{{\"edges\":{}1{}}}", "[".repeat(70), "]".repeat(70));
        let bad = [
            "{",
            r#"{"edges":[1,]}"#,
            r#"{"edges":tru}"#,
            r#"{"edges":"\u12"}"#,
            r#"{"a":1} x"#,
            r#"{"edges":[1}"#,
            deep.as_str(),
        ];
        for t in bad {
            assert!(
                parse(t.as_bytes()).is_err() && parse_members(t.as_bytes(), &keep).is_err(),
                "{t}"
            );
        }
        assert!(parse_members(
            &[b'{', b'"', b'e', b'"', b':', b'"', 0xff, b'"', b'}'],
            &keep
        )
        .is_err());
    }
}
