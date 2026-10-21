//! A model that answers from a recording (`llmreg/1`, one JSON line per turn) so the agent and the page are tested without network. In `strict`
//! mode the SHA-256 of the request body must match the recorded one.

use mzlab_core::budget::sha256_hex;
use mzlab_core::error::{Error, Result};
use mzlab_core::json::{parse, Out, Value};

use crate::event::Event;
use crate::provider::Request;

struct Turn {
    sha: Option<String>,
    events: Vec<Event>,
}

pub struct Replay {
    turns: Vec<Turn>,
    strict: bool,
    next: usize,
}

/// The hash a recording keeps for a request: the SHA-256 of the body (the URL and the headers are the descriptor's, the key is not there).
pub fn request_sha256(r: &Request) -> String {
    sha256_hex(r.body.as_bytes())
}

/// One recording line for a turn.
pub fn record_line(turn: usize, req: &Request, events: &[Event]) -> String {
    let mut o = Out::default();
    o.raw("{\"events\":[");
    for (i, e) in events.iter().enumerate() {
        if i > 0 {
            o.raw(",");
        }
        o.raw(&e.to_json());
    }
    o.raw("],\"request_sha256\":")
        .str(&request_sha256(req))
        .raw(",\"schema\":\"llmreg/1\",\"turn\":")
        .int(turn as i64)
        .raw("}");
    o.0
}

impl Replay {
    pub fn parse(text: &str, strict: bool) -> Result<Replay> {
        let bad = |line: usize, f: &str| {
            Error::new("err.llm.replay.format")
                .with("line", line)
                .with("field", f)
        };
        let mut turns = Vec::new();
        for (n, l) in text
            .lines()
            .enumerate()
            .filter(|(_, l)| !l.trim().is_empty())
        {
            let v = parse(l.as_bytes()).map_err(|_| bad(n + 1, "json"))?;
            if v.get("schema").and_then(Value::str) != Some("llmreg/1") {
                return Err(bad(n + 1, "schema"));
            }
            let events = v
                .get("events")
                .map_or(&[][..], Value::arr)
                .iter()
                .map(Event::from_value)
                .collect::<Result<Vec<_>>>()?;
            let sha = v
                .get("request_sha256")
                .and_then(Value::str)
                .map(str::to_string);
            if strict && sha.is_none() {
                return Err(bad(n + 1, "request_sha256"));
            }
            turns.push(Turn { sha, events });
        }
        Ok(Replay {
            turns,
            strict,
            next: 0,
        })
    }

    pub fn len(&self) -> usize {
        self.turns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.turns.is_empty()
    }

    /// Starts again from the first turn.
    pub fn reset(&mut self) {
        self.next = 0;
    }

    /// The events of the next turn.
    pub fn respond(&mut self, req: &Request) -> Result<Vec<Event>> {
        let e = self.at(self.next, req)?;
        self.next += 1;
        Ok(e)
    }

    /// The events of turn `turn`.
    pub fn at(&self, turn: usize, req: &Request) -> Result<Vec<Event>> {
        let t = self
            .turns
            .get(turn)
            .ok_or_else(|| Error::new("err.llm.replay.exhausted").with("turn", turn))?;
        if let (true, Some(want)) = (self.strict, &t.sha) {
            let got = request_sha256(req);
            if &got != want {
                return Err(Error::new("err.llm.replay.mismatch")
                    .with("turn", turn)
                    .with("expected", want)
                    .with("got", got));
            }
        }
        Ok(t.events.clone())
    }
}
