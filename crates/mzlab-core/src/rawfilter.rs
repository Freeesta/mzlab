//! A filter over the mzML that `opentfraw` writes for a `.raw`, line by line, as it goes out (the file is never held in memory):
//! - the `spectrumRef` of a precursor names the scan it was selected from: the writer takes it from the trailer field "Master Index",
//!   which is not a scan number (it reads 1 for every MS2 of a data-dependent run), so a reference of 1 is replaced by the last
//!   scan of the level above that came before; any other reference is kept (it comes from a real "Master Scan Number");
//! - the personal data of the acquisition (computer, user folders and the paths of the files and of the methods, the values of the
//!   user labels) are left out of the `<run>` when `anonymous` is set; they stay in the `.raw`.

use std::io::{self, Write};

/// `userParam` names (after `opentfraw.`) that identify a person, a computer or a path.
const IDENTITY: [&str; 5] = [
    "computer_name",
    "original_file_name",
    "original_file_path",
    "instrument_method_file",
    "processing_method_file",
];

pub struct MzmlFilter<W: Write> {
    inner: W,
    line: Vec<u8>,
    anonymous: bool,
    /// Scan number of the spectrum being written, and the last scan seen at each level (index = level).
    current: u32,
    last: [u32; 8],
}

impl<W: Write> MzmlFilter<W> {
    pub fn new(inner: W, anonymous: bool) -> Self {
        MzmlFilter {
            inner,
            line: Vec::new(),
            anonymous,
            current: 0,
            last: [0; 8],
        }
    }

    fn emit(&mut self) -> io::Result<()> {
        let line = std::mem::take(&mut self.line);
        let text = String::from_utf8_lossy(&line);
        if let Some(out) = self.rewrite(&text) {
            self.inner.write_all(out.as_bytes())?;
        }
        Ok(())
    }

    /// The line to write instead of `l` (with its end of line), or `None` to drop it.
    fn rewrite(&mut self, l: &str) -> Option<String> {
        let t = l.trim_start();
        if t.starts_with("<spectrum ") {
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
        } else if self.anonymous && t.starts_with("<userParam name=\"opentfraw.") {
            let name = t
                .split('"')
                .nth(1)
                .unwrap_or("")
                .trim_start_matches("opentfraw.");
            if IDENTITY.contains(&name)
                || (name.starts_with("user_label.") && name.ends_with(".value"))
            {
                return None;
            }
        }
        Some(format!("{l}\n"))
    }
}

fn number_after(s: &str, key: &str) -> Option<u32> {
    let rest = &s[s.find(key)? + key.len()..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

fn value_of(s: &str) -> Option<&str> {
    s.split("value=\"").nth(1)?.split('"').next()
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

#[cfg(test)]
mod tests {
    use super::*;

    fn run(xml: &str, anonymous: bool) -> String {
        let mut out = Vec::new();
        {
            let mut f = MzmlFilter::new(&mut out, anonymous);
            for part in xml.as_bytes().chunks(7) {
                f.write_all(part).unwrap(); // pieces cut inside the lines
            }
            f.flush().unwrap();
        }
        String::from_utf8(out).unwrap()
    }

    fn spectrum(scan: u32, level: u32, parent: Option<u32>) -> String {
        let p = parent.map_or(String::new(), |p| format!("<precursor spectrumRef=\"controllerType=0 controllerNumber=1 scan={p}\">\n<selectedIon/>\n</precursor>\n"));
        format!("<spectrum id=\"controllerType=0 controllerNumber=1 scan={scan}\" index=\"0\">\n<cvParam cvRef=\"MS\" accession=\"MS:1000511\" name=\"ms level\" value=\"{level}\"/>\n{p}</spectrum>\n")
    }

    #[test]
    fn a_precursor_that_points_to_scan_1_points_to_the_survey_scan_before_it() {
        let xml = [
            spectrum(1, 1, None),
            spectrum(2, 2, Some(1)),
            spectrum(3, 2, Some(1)),
            spectrum(4, 1, None),
            spectrum(5, 2, Some(1)),
            spectrum(6, 3, Some(1)),
        ]
        .concat();
        let got = run(&xml, false);
        let refs: Vec<u32> = got
            .lines()
            .filter(|l| l.contains("spectrumRef"))
            .map(|l| number_after(l, "scan=").unwrap())
            .collect();
        assert_eq!(refs, [1, 1, 4, 5]); // the MS2 of scan 5 follows the survey scan 4, the MS3 its MS2 (scan 5)
    }

    #[test]
    fn a_real_reference_is_kept() {
        let xml = [
            spectrum(1, 1, None),
            spectrum(2, 1, None),
            spectrum(3, 1, None),
            spectrum(4, 2, Some(2)),
        ]
        .concat();
        let got = run(&xml, false);
        assert!(got.contains("scan=2\">"), "a reference other than 1 stays");
        assert_eq!(got.lines().count(), xml.lines().count());
    }

    #[test]
    fn personal_data_are_left_out_only_when_asked() {
        let xml = "<run>\n    <userParam name=\"opentfraw.computer_name\" value=\"X\"/>\n    <userParam name=\"opentfraw.raw_version\" value=\"66\"/>\n    <userParam name=\"opentfraw.original_file_path\" value=\"C:\\a\"/>\n    <userParam name=\"opentfraw.user_label.0.heading\" value=\"Study\"/>\n    <userParam name=\"opentfraw.user_label.0.value\" value=\"P\"/>\n</run>\n";
        assert_eq!(run(xml, false), xml);
        let got = run(xml, true);
        assert!(
            !got.contains("computer_name")
                && !got.contains("original_file_path")
                && !got.contains("label.0.value")
        );
        assert!(got.contains("raw_version") && got.contains("label.0.heading"));
    }
}
