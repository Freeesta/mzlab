//! Builders for report values (`json::Value` trees) and their text: the Rust side of `ionfamily.to_jsonable` (NaN and infinities
//! become `null`) and of `i18n.message` (`{key, params}`).

use crate::json::{Out, Value};

/// A number; NaN and infinities become `null`.
pub fn num(v: f64) -> Value {
    if v.is_finite() {
        Value::Num(v)
    } else {
        Value::Null
    }
}
pub fn int(v: i64) -> Value {
    Value::Num(v as f64)
}
pub fn floats(a: &[f64]) -> Value {
    Value::Arr(a.iter().map(|&v| num(v)).collect())
}
pub fn text(s: &str) -> Value {
    Value::Str(s.to_string())
}
pub fn opt(v: Option<f64>) -> Value {
    v.map_or(Value::Null, num)
}

/// `message(key, **params)`: `{"key": ..., "params": {...}}`.
pub fn msg(key: &str, params: &[(&str, Value)]) -> Value {
    Obj::new().put("key", text(key)).put("params", params.iter().fold(Obj::new(), |o, (k, v)| o.put(k, v.clone())).done()).done()
}

/// An object under construction (keys in insertion order, like a Python dict).
#[derive(Default)]
pub struct Obj(Vec<(String, Value)>);

impl Obj {
    pub fn new() -> Obj {
        Obj(Vec::new())
    }
    pub fn put(mut self, k: &str, v: Value) -> Obj {
        self.set(k, v);
        self
    }
    pub fn set(&mut self, k: &str, v: Value) {
        match self.0.iter_mut().find(|(n, _)| n == k) {
            Some(slot) => slot.1 = v,
            None => self.0.push((k.to_string(), v)),
        }
    }
    pub fn n(self, k: &str, v: f64) -> Obj {
        self.put(k, num(v))
    }
    pub fn i(self, k: &str, v: i64) -> Obj {
        self.put(k, int(v))
    }
    pub fn b(self, k: &str, v: bool) -> Obj {
        self.put(k, Value::Bool(v))
    }
    pub fn done(self) -> Value {
        Value::Obj(self.0)
    }
}

fn write_into(o: &mut Out, v: &Value) {
    match v {
        Value::Null => {
            o.raw("null");
        }
        Value::Bool(b) => {
            o.raw(if *b { "true" } else { "false" });
        }
        Value::Num(n) => {
            o.num(*n);
        }
        Value::Str(s) => {
            o.str(s);
        }
        Value::Arr(a) => {
            o.raw("[");
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    o.raw(",");
                }
                write_into(o, x);
            }
            o.raw("]");
        }
        Value::Obj(m) => {
            o.raw("{");
            for (i, (k, x)) in m.iter().enumerate() {
                if i > 0 {
                    o.raw(",");
                }
                o.str(k).raw(":");
                write_into(o, x);
            }
            o.raw("}");
        }
    }
}

/// The JSON text of a value.
pub fn write(v: &Value) -> String {
    let mut o = Out::default();
    write_into(&mut o, v);
    o.0
}
