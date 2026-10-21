//! A server-sent-events reader that takes bytes cut anywhere (inside a line, inside a UTF-8 character) and returns the complete frames.

#[derive(Debug, PartialEq)]
pub struct Frame {
    pub name: String,
    pub data: String,
}

#[derive(Default)]
pub struct Sse {
    buf: Vec<u8>,
    name: String,
    data: String,
    has_data: bool,
}

impl Sse {
    pub fn push(&mut self, bytes: &[u8]) -> Vec<Frame> {
        self.buf.extend_from_slice(bytes);
        let mut out = Vec::new();
        while let Some(p) = self.buf.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = self.buf.drain(..=p).collect();
            self.line(&line[..p], &mut out);
        }
        out
    }

    /// The end of the stream: a last line without newline and a frame without its blank line still count.
    pub fn finish(&mut self) -> Vec<Frame> {
        let mut out = Vec::new();
        if !self.buf.is_empty() {
            let line = std::mem::take(&mut self.buf);
            self.line(&line, &mut out);
        }
        self.dispatch(&mut out);
        out
    }

    fn line(&mut self, raw: &[u8], out: &mut Vec<Frame>) {
        let raw = raw.strip_suffix(b"\r").unwrap_or(raw);
        let line = String::from_utf8_lossy(raw);
        if line.is_empty() {
            self.dispatch(out);
        } else if !line.starts_with(':') {
            let (field, value) = line.split_once(':').unwrap_or((&line, ""));
            let value = value.strip_prefix(' ').unwrap_or(value);
            match field {
                "event" => self.name = value.to_string(),
                "data" => {
                    if self.has_data {
                        self.data.push('\n');
                    }
                    self.data.push_str(value);
                    self.has_data = true;
                }
                _ => {}
            }
        }
    }

    fn dispatch(&mut self, out: &mut Vec<Frame>) {
        if self.has_data {
            out.push(Frame {
                name: std::mem::take(&mut self.name),
                data: std::mem::take(&mut self.data),
            });
        }
        self.name.clear();
        self.data.clear();
        self.has_data = false;
    }
}
