//! The log of a conversation (`llmlog/1`): one JSON line per event with sorted keys, built in memory (the caller writes it). A line that looks like it
//! holds an API key is refused: keys never reach a log, a diagnosis or a recording.

use mzlab_core::error::{Error, Result};

use crate::event::{join, Event};

#[derive(Default)]
pub struct Log {
    text: String,
    seq: u64,
}

/// True when `s` holds `sk-`, `sk-ant-` or `AIza` at the start of a word and followed by something (so «task-force» does not trip it).
pub fn has_secret(s: &str) -> bool {
    let b = s.as_bytes();
    ["sk-", "AIza"].iter().any(|p| {
        s.match_indices(p).any(|(i, _)| {
            let before = i == 0 || !b[i - 1].is_ascii_alphanumeric();
            let after = b
                .get(i + p.len())
                .is_some_and(|c| c.is_ascii_alphanumeric());
            before && after
        })
    })
}

impl Log {
    pub fn new() -> Log {
        Log::default()
    }

    /// Adds the line of `event` in turn `turn`.
    pub fn event(&mut self, turn: u32, event: &Event) -> Result<()> {
        let mut f = event.fields();
        f.push(("schema", "\"llmlog/1\"".to_string()));
        f.push(("seq", self.seq.to_string()));
        f.push(("turn", turn.to_string()));
        f.sort_by_key(|(k, _)| *k);
        let line = join(&f);
        if has_secret(&line) {
            return Err(Error::new("err.llm.log.secret").with("seq", self.seq));
        }
        self.text.push_str(&line);
        self.text.push('\n');
        self.seq += 1;
        Ok(())
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    /// The last line written, without its newline.
    pub fn last(&self) -> &str {
        let t = self.text.trim_end_matches('\n');
        t.rsplit('\n').next().unwrap_or("")
    }

    /// Adds a line of kind `kind` (`call`, `decision`, ...) in turn `turn`; the values of `fields` are JSON text.
    pub fn record(
        &mut self,
        turn: u32,
        kind: &str,
        fields: &[(&'static str, String)],
    ) -> Result<()> {
        let mut f = fields.to_vec();
        f.push(("ev", format!("\"{kind}\"")));
        f.push(("schema", "\"llmlog/1\"".to_string()));
        f.push(("seq", self.seq.to_string()));
        f.push(("turn", turn.to_string()));
        f.sort_by_key(|(k, _)| *k);
        let line = join(&f);
        if has_secret(&line) {
            return Err(Error::new("err.llm.log.secret").with("seq", self.seq));
        }
        self.text.push_str(&line);
        self.text.push('\n');
        self.seq += 1;
        Ok(())
    }
}
