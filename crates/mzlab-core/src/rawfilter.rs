//! A filter over the mzML that `opentfraw` writes for a `.raw`, line by line, as it goes out (the file is never held in memory):
//! - the `spectrumRef` of a precursor names the scan it was selected from: the writer takes it from the trailer field "Master Index",
//!   which is not a scan number (it reads 1 for every MS2 of a data-dependent run), so a reference of 1 is replaced by the last
//!   scan of the level above that came before; any other reference is kept (it comes from a real "Master Scan Number");

use std::io::{self, Write};

pub struct MzmlFilter<W: Write> {
    inner: W,
    line: Vec<u8>,
    /// Lines written right after the `<run ...>` tag (the `metodo/1` document as a `userParam`).
    run_extra: Option<String>,
    /// Inside a `pressure chromatogram` (MS:1003019): its intensity array gets the unit.
    pressure: bool,
    /// Scan number of the spectrum being written, and the last scan seen at each level (index = level).
    current: u32,
    last: [u32; 8],
}

fn number_after(s: &str, key: &str) -> Option<u32> {
    let rest = &s[s.find(key)? + key.len()..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

impl<W: Write> MzmlFilter<W> {
    pub fn new(inner: W) -> Self {
        MzmlFilter {
            inner,
            line: Vec::new(),
            run_extra: None,
            pressure: false,
            current: 0,
            last: [0; 8],
        }
    }

    /// Writes `extra` (complete lines) after the `<run ...>` tag.
    pub fn with_run_extra(mut self, extra: String) -> Self {
        self.run_extra = Some(extra);
        self
    }

    fn emit(&mut self) -> io::Result<()> {
        let line = std::mem::take(&mut self.line);
        let text = String::from_utf8_lossy(&line);
        if let Some(out) = self.rewrite(&text) {
            self.inner.write_all(out.as_bytes())?;
        }
        Ok(())
    }
}

impl<W: Write> Write for MzmlFilter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        for chunk in buf.split_inclusive(|&b| b == b'\n') {
            if chunk.last() == Some(&b'\n') {
                self.line.extend_from_slice(&chunk[..chunk.len() - 1]);
                self.emit()?;
            } else {
                self.line.extend_from_slice(chunk);
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if !self.line.is_empty() {
            self.emit()?;
        }
        self.inner.flush()
    }
}

fn value_of(s: &str) -> Option<&str> {
    s.split("value=\"").nth(1)?.split('"').next()
}

impl<W: Write> MzmlFilter<W> {
    /// The line to write instead of `l` (with its end of line), or `None` to drop it.
    fn rewrite(&mut self, l: &str) -> Option<String> {
        let t = l.trim_start();
        if t.starts_with("<run ") {
            if let Some(extra) = self.run_extra.take() {
                return Some(format!("{l}\n{extra}\n"));
            }
        } else if t.starts_with("<chromatogram ") {
            self.pressure = false;
        } else if t.contains("accession=\"MS:1003019\"") {
            self.pressure = true;
        } else if self.pressure && t.contains("accession=\"MS:1000515\"") {
            // the writer gives no unit to an intensity array; the pressures are written in pascal
            let unit = "unitCvRef=\"UO\" unitAccession=\"UO:0000110\" unitName=\"pascal\"";
            return Some(format!(
                "{}\n",
                l.replacen("value=\"\"", &format!("value=\"\" {unit}"), 1)
            ));
        } else if t.starts_with("<spectrum ") {
            self.current = number_after(t, "scan=").unwrap_or(0);
        } else if t.contains("accession=\"MS:1000511\"") && !t.contains("<precursor") {
            if let Some(level) = value_of(t).and_then(|v| v.parse::<usize>().ok()) {
                self.last[level.min(7)] = self.current;
                // the references of this spectrum are to the level above
                self.last[0] = level as u32;
            }
        } else if t.starts_with("<precursor ") && t.contains("spectrumRef=") {
            let level = self.last[0] as usize;
            let parent = if level >= 2 {
                self.last[(level - 1).min(7)]
            } else {
                0
            };
            if number_after(t, "spectrumRef=\"controllerType=0 controllerNumber=1 scan=") == Some(1)
                && parent > 1
            {
                return Some(format!(
                    "{}\n",
                    l.replacen("scan=1\"", &format!("scan={parent}\""), 1)
                ));
            }
        }
        Some(format!("{l}\n"))
    }
}

#[cfg(test)]
mod tests;
