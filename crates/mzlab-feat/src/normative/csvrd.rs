//! A streaming CSV/TSV reader for the large inputs (SusDat is 124 MB): records of fields, quotes with doubled quotes, line breaks inside quotes.

use std::io::{BufRead, BufReader, Read};

pub struct Records<R: Read> {
    r: BufReader<R>,
    sep: u8,
    buf: Vec<u8>,
}

impl<R: Read> Records<R> {
    pub fn new(r: R, sep: u8) -> Records<R> {
        Records {
            r: BufReader::with_capacity(1 << 20, r),
            sep,
            buf: Vec::new(),
        }
    }
}

impl<R: Read> Iterator for Records<R> {
    type Item = Vec<String>;

    fn next(&mut self) -> Option<Vec<String>> {
        let mut fields: Vec<String> = Vec::new();
        let mut cur: Vec<u8> = Vec::new();
        let mut quoted = false;
        let mut any = false;
        loop {
            self.buf.clear();
            let n = self.r.read_until(b'\n', &mut self.buf).ok()?;
            if n == 0 {
                if !any {
                    return None;
                }
                break;
            }
            any = true;
            let line = if !quoted && self.buf.ends_with(b"\r\n") {
                &self.buf[..n - 2]
            } else {
                &self.buf[..n]
            };
            let mut i = 0;
            while i < line.len() {
                let c = line[i];
                if quoted {
                    if c == b'"' {
                        if line.get(i + 1) == Some(&b'"') {
                            cur.push(b'"');
                            i += 1;
                        } else {
                            quoted = false;
                        }
                    } else {
                        cur.push(c);
                    }
                } else if c == b'"' && cur.is_empty() {
                    quoted = true;
                } else if c == self.sep {
                    fields.push(String::from_utf8_lossy(&cur).into_owned());
                    cur.clear();
                } else if c == b'\n' || c == b'\r' {
                    // end of record
                } else {
                    cur.push(c);
                }
                i += 1;
            }
            if !quoted {
                break;
            }
        }
        fields.push(String::from_utf8_lossy(&cur).into_owned());
        Some(fields)
    }
}

/// The separator of a text: the most frequent of `;`, tab and `,` in its first line.
pub fn guess_sep(first_line: &str) -> u8 {
    b";\t,"
        .iter()
        .copied()
        .max_by_key(|s| first_line.bytes().filter(|b| b == s).count())
        .unwrap_or(b',')
}

/// The index of the first header that equals one of `names` (any case), if any.
pub fn col(header: &[String], names: &[&str]) -> Option<usize> {
    header.iter().position(|h| {
        names.iter().any(|n| {
            h.trim()
                .trim_start_matches('\u{feff}')
                .eq_ignore_ascii_case(n)
        })
    })
}

/// A field of a record, empty when the record is short.
pub fn at(rec: &[String], i: Option<usize>) -> &str {
    i.and_then(|i| rec.get(i)).map_or("", |s| s.trim())
}
