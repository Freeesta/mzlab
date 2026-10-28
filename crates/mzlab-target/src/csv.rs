// RFC 4180 CSV parser and target entry deserializer.
use std::borrow::Cow;
use std::fmt;
use std::io::Read;

use crate::entry::{CheckFlags, TargetEntry};

/// Errors encountered while reading or parsing target CSV data.
#[derive(Debug)]
pub enum ParseError {
    /// Standard I/O read failure.
    Io(std::io::Error),
    /// RFC 4180 format violation (e.g. unclosed quotation mark).
    CsvFormat { line: usize, message: String },
    /// Required header column not present in the CSV file.
    MissingHeader(String),
    /// Record missing expected column at the given line number.
    MissingColumn { line: usize, column: String },
    /// Float conversion failed for mass or m/z column.
    InvalidFloat {
        line: usize,
        column: String,
        value: String,
    },
    /// Integer conversion failed for identifier column.
    InvalidInteger {
        line: usize,
        column: String,
        value: String,
    },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(f, "I/O error reading target CSV: {err}"),
            Self::CsvFormat { line, message } => {
                write!(f, "RFC 4180 format error at line {line}: {message}")
            }
            Self::MissingHeader(col) => write!(f, "Missing required CSV header column: {col}"),
            Self::MissingColumn { line, column } => {
                write!(f, "Missing expected column '{column}' at line {line}")
            }
            Self::InvalidFloat {
                line,
                column,
                value,
            } => {
                write!(
                    f,
                    "Invalid float '{value}' for column '{column}' at line {line}"
                )
            }
            Self::InvalidInteger {
                line,
                column,
                value,
            } => {
                write!(
                    f,
                    "Invalid integer '{value}' for column '{column}' at line {line}"
                )
            }
        }
    }
}

impl std::error::Error for ParseError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

/// A tokenized CSV text: every field of every record, fields borrowed from the text unless a quote had to be unescaped or joined.
struct Records<'a> {
    fields: Vec<Cow<'a, str>>,
    /// End of each record in `fields`.
    ends: Vec<usize>,
}

impl<'a> Records<'a> {
    fn len(&self) -> usize {
        self.ends.len()
    }

    fn row(&self, k: usize) -> &[Cow<'a, str>] {
        let a = if k == 0 { 0 } else { self.ends[k - 1] };
        &self.fields[a..self.ends[k]]
    }
}

/// How a field ended: by a comma, by a line break, or by the end of the text.
enum FieldEnd {
    Comma,
    Line,
    Eof,
}

/// Position of the first field separator or line break in `b` from `from`.
fn field_stop(b: &[u8], from: usize) -> usize {
    b[from..]
        .iter()
        .position(|&c| matches!(c, b',' | b'\r' | b'\n'))
        .map_or(b.len(), |k| from + k)
}

/// Read the field that starts at `*i`, leave `*i` after its terminator and count line breaks in `line`.
/// A field opens a quote only at its first byte; any later quote is data, and text after a closing quote is joined to the quoted text.
fn read_field<'a>(
    s: &'a str,
    i: &mut usize,
    line: &mut usize,
) -> Result<(Cow<'a, str>, FieldEnd), ParseError> {
    let b = s.as_bytes();
    let start = *i;
    let (field, stop) = if b.get(start) == Some(&b'"') {
        let mut owned = String::new();
        let mut seg = start + 1;
        let close = loop {
            let Some(k) = b[seg..].iter().position(|&c| c == b'"') else {
                *line += b[seg..].iter().filter(|&&c| c == b'\n').count();
                return Err(ParseError::CsvFormat {
                    line: *line,
                    message: "Unclosed quote at end of CSV".to_string(),
                });
            };
            let q = seg + k;
            *line += b[seg..q].iter().filter(|&&c| c == b'\n').count();
            if b.get(q + 1) == Some(&b'"') {
                owned.push_str(&s[seg..q]);
                owned.push('"');
                seg = q + 2;
            } else {
                break q;
            }
        };
        let stop = field_stop(b, close + 1);
        let field = if owned.is_empty() && stop == close + 1 {
            Cow::Borrowed(&s[seg..close])
        } else {
            owned.push_str(&s[seg..close]);
            owned.push_str(&s[close + 1..stop]);
            Cow::Owned(owned)
        };
        (field, stop)
    } else {
        let stop = field_stop(b, start);
        (Cow::Borrowed(&s[start..stop]), stop)
    };
    let (end, next) = match b.get(stop) {
        None => (FieldEnd::Eof, stop),
        Some(b',') => (FieldEnd::Comma, stop + 1),
        Some(&c) => {
            *line += 1;
            let crlf = c == b'\r' && b.get(stop + 1) == Some(&b'\n');
            (FieldEnd::Line, stop + 1 + usize::from(crlf))
        }
    };
    *i = next;
    Ok((field, end))
}

/// Split CSV text into records and fields (RFC 4180, tolerant of quotes in the middle of a field); a leading byte order mark is dropped.
fn tokenize(content: &str) -> Result<Records<'_>, ParseError> {
    let s = content.strip_prefix('\u{feff}').unwrap_or(content);
    let mut rec = Records {
        fields: Vec::new(),
        ends: Vec::new(),
    };
    let (mut i, mut line) = (0usize, 1usize);
    while i < s.len() {
        loop {
            let (field, end) = read_field(s, &mut i, &mut line)?;
            rec.fields.push(field);
            if !matches!(end, FieldEnd::Comma) {
                break;
            }
        }
        rec.ends.push(rec.fields.len());
    }
    Ok(rec)
}

/// Tokenize and parse raw CSV text adhering to RFC 4180 standards.
pub fn parse_rfc4180_str(content: &str) -> Result<Vec<Vec<String>>, ParseError> {
    let rec = tokenize(content)?;
    Ok((0..rec.len())
        .map(|k| rec.row(k).iter().map(|f| f.to_string()).collect())
        .collect())
}

struct HeaderIndices {
    id: usize,
    name_en: usize,
    name_it: usize,
    neutral_formula: usize,
    monoisotopic_mass: usize,
    mz_mh: usize,
    mz_mna: usize,
    mz_mnh4: usize,
    smiles: usize,
    inchikey: usize,
    inchikey_connectivity: usize,
    cas: usize,
    pubchem_cid: usize,
    effect_class: usize,
    chemical_class: usize,
    role: usize,
    psychoactive: usize,
    synonyms: usize,
    check: Option<usize>,
}

impl HeaderIndices {
    fn from_header(header: &[Cow<'_, str>]) -> Result<Self, ParseError> {
        let find = |name: &str| -> Result<usize, ParseError> {
            header
                .iter()
                .position(|h| h.trim().eq_ignore_ascii_case(name))
                .ok_or_else(|| ParseError::MissingHeader(name.to_string()))
        };
        let find_opt = |name: &str| -> Option<usize> {
            header
                .iter()
                .position(|h| h.trim().eq_ignore_ascii_case(name))
        };

        Ok(Self {
            id: find("id")?,
            name_en: find("name_en")?,
            name_it: find("name_it")?,
            neutral_formula: find("neutral_formula")?,
            monoisotopic_mass: find("monoisotopic_mass")?,
            mz_mh: find("mz_mh")?,
            mz_mna: find("mz_mna")?,
            mz_mnh4: find("mz_mnh4")?,
            smiles: find("smiles")?,
            inchikey: find("inchikey")?,
            inchikey_connectivity: find("inchikey_connectivity")?,
            cas: find("cas")?,
            pubchem_cid: find("pubchem_cid")?,
            effect_class: find("effect_class")?,
            chemical_class: find("chemical_class")?,
            role: find("role")?,
            psychoactive: find("psychoactive")?,
            synonyms: find("synonyms")?,
            check: find_opt("check"),
        })
    }
}

fn parse_float_col(
    row: &[Cow<'_, str>],
    idx: usize,
    col: &'static str,
    line: usize,
) -> Result<f64, ParseError> {
    let raw = row
        .get(idx)
        .map(|s| s.trim())
        .ok_or_else(|| ParseError::MissingColumn {
            line,
            column: col.to_string(),
        })?;
    raw.parse::<f64>().map_err(|_| ParseError::InvalidFloat {
        line,
        column: col.to_string(),
        value: raw.to_string(),
    })
}

struct RowMeta {
    cas: Option<String>,
    pubchem_cid: Option<u64>,
    psychoactive: bool,
    synonyms: Vec<String>,
    check: Option<String>,
    check_flags: CheckFlags,
}

fn parse_meta_fields(
    row: &[Cow<'_, str>],
    h: &HeaderIndices,
    line: usize,
) -> Result<RowMeta, ParseError> {
    let get_str = |idx: usize, col: &'static str| -> Result<&str, ParseError> {
        row.get(idx)
            .map(|s| s.trim())
            .ok_or_else(|| ParseError::MissingColumn {
                line,
                column: col.to_string(),
            })
    };

    let cas_raw = get_str(h.cas, "cas")?;
    let cas = if cas_raw.is_empty() {
        None
    } else {
        Some(cas_raw.to_string())
    };

    let cid_raw = get_str(h.pubchem_cid, "pubchem_cid")?;
    let pubchem_cid = if cid_raw.is_empty() {
        None
    } else {
        let val = cid_raw
            .parse::<u64>()
            .map_err(|_| ParseError::InvalidInteger {
                line,
                column: "pubchem_cid".to_string(),
                value: cid_raw.to_string(),
            })?;
        Some(val)
    };

    let psycho_raw = get_str(h.psychoactive, "psychoactive")?;
    let psychoactive = matches!(
        psycho_raw.to_ascii_lowercase().as_str(),
        "yes" | "true" | "1" | "sì" | "si"
    );

    let syn_raw = get_str(h.synonyms, "synonyms")?;
    let synonyms = if syn_raw.is_empty() {
        Vec::new()
    } else {
        syn_raw
            .split('|')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect()
    };

    let check = h
        .check
        .and_then(|idx| row.get(idx))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let check_flags = check
        .as_deref()
        .map(CheckFlags::from_check_str)
        .unwrap_or_default();

    Ok(RowMeta {
        cas,
        pubchem_cid,
        psychoactive,
        synonyms,
        check,
        check_flags,
    })
}

fn parse_row(row: &[Cow<'_, str>], h: &HeaderIndices, line: usize) -> Result<TargetEntry, ParseError> {
    let get_str = |idx: usize, col: &'static str| -> Result<String, ParseError> {
        row.get(idx)
            .map(|s| s.trim().to_string())
            .ok_or_else(|| ParseError::MissingColumn {
                line,
                column: col.to_string(),
            })
    };

    let monoisotopic_mass = parse_float_col(row, h.monoisotopic_mass, "monoisotopic_mass", line)?;
    let mz_mh = parse_float_col(row, h.mz_mh, "mz_MH", line)?;
    let mz_mna = parse_float_col(row, h.mz_mna, "mz_MNa", line)?;
    let mz_mnh4 = parse_float_col(row, h.mz_mnh4, "mz_MNH4", line)?;
    let meta = parse_meta_fields(row, h, line)?;

    Ok(TargetEntry {
        id: get_str(h.id, "id")?,
        name_en: get_str(h.name_en, "name_en")?,
        name_it: get_str(h.name_it, "name_it")?,
        neutral_formula: get_str(h.neutral_formula, "neutral_formula")?,
        monoisotopic_mass,
        mz_mh,
        mz_mna,
        mz_mnh4,
        smiles: get_str(h.smiles, "smiles")?,
        inchikey: get_str(h.inchikey, "inchikey")?,
        inchikey_connectivity: get_str(h.inchikey_connectivity, "inchikey_connectivity")?,
        cas: meta.cas,
        pubchem_cid: meta.pubchem_cid,
        effect_class: get_str(h.effect_class, "effect_class")?,
        chemical_class: get_str(h.chemical_class, "chemical_class")?,
        role: get_str(h.role, "role")?,
        psychoactive: meta.psychoactive,
        synonyms: meta.synonyms,
        check: meta.check,
        check_flags: meta.check_flags,
    })
}

/// Parse target entries from an RFC 4180 CSV reader.
pub fn parse_target_csv(mut reader: impl Read) -> Result<Vec<TargetEntry>, ParseError> {
    let mut content = String::new();
    reader
        .read_to_string(&mut content)
        .map_err(ParseError::Io)?;
    let records = tokenize(&content)?;

    if records.len() == 0 {
        return Err(ParseError::CsvFormat {
            line: 0,
            message: "Empty CSV input".to_string(),
        });
    }
    let indices = HeaderIndices::from_header(records.row(0))?;
    let mut entries = Vec::with_capacity(records.len() - 1);

    for k in 1..records.len() {
        let line_num = k + 1;
        let row = records.row(k);
        // Skip trailing empty lines
        if row.is_empty() || (row.len() == 1 && row[0].trim().is_empty()) {
            continue;
        }
        entries.push(parse_row(row, &indices, line_num)?);
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The character-by-character tokenizer this module used before the borrowing one: the fuzz test below holds the new one to the same records.
    mod reference {
    pub fn parse_old(content: &str) -> Result<Vec<Vec<String>>, super::ParseError> {
        let mut records = Vec::new();
        let mut current_record = Vec::new();
        let mut current_field = String::new();
        let mut in_quotes = false;
        let mut chars = content.chars().peekable();
        let mut line_num = 1usize;
        let mut field_started = false;

        while let Some(c) = chars.next() {
            if in_quotes {
                if c == '"' {
                    if let Some(&'"') = chars.peek() {
                        chars.next();
                        current_field.push('"');
                    } else {
                        in_quotes = false;
                    }
                } else {
                    if c == '\n' {
                        line_num += 1;
                    }
                    current_field.push(c);
                }
            } else {
                match c {
                    '"' => {
                        if !field_started {
                            in_quotes = true;
                            field_started = true;
                        } else {
                            current_field.push('"');
                        }
                    }
                    ',' => {
                        current_record.push(std::mem::take(&mut current_field));
                        field_started = false;
                    }
                    '\r' => {
                        if let Some(&'\n') = chars.peek() {
                            chars.next();
                        }
                        line_num += 1;
                        current_record.push(std::mem::take(&mut current_field));
                        field_started = false;
                        records.push(std::mem::take(&mut current_record));
                    }
                    '\n' => {
                        line_num += 1;
                        current_record.push(std::mem::take(&mut current_field));
                        field_started = false;
                        records.push(std::mem::take(&mut current_record));
                    }
                    _ => {
                        field_started = true;
                        current_field.push(c);
                    }
                }
            }
        }

        if in_quotes {
            return Err(super::ParseError::CsvFormat {
                line: line_num,
                message: "Unclosed quote at end of CSV".to_string(),
            });
        }

        if field_started || !current_field.is_empty() || !current_record.is_empty() {
            current_record.push(current_field);
            records.push(current_record);
        }

        Ok(records)
    }
    }

    fn rows(s: &str) -> Vec<Vec<String>> {
        parse_rfc4180_str(s).unwrap()
    }

    #[test]
    fn tokenizer_matches_the_reference_on_random_text() {
        let alphabet = ['a', 'b', ',', ',', '"', '"', '\r', '\n', ' ', 'é', '|'];
        let mut x = 0x2545F4914F6CDD1Du64;
        for _ in 0..40_000 {
            let n = {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                (x % 24) as usize
            };
            let text: String = (0..n)
                .map(|_| {
                    x ^= x << 13;
                    x ^= x >> 7;
                    x ^= x << 17;
                    alphabet[(x % alphabet.len() as u64) as usize]
                })
                .collect();
            match (reference::parse_old(&text), parse_rfc4180_str(&text)) {
                (Ok(a), Ok(b)) => assert_eq!(a, b, "{text:?}"),
                (Err(a), Err(b)) => assert_eq!(format!("{a}"), format!("{b}"), "{text:?}"),
                (a, b) => panic!("{text:?}: {a:?} vs {b:?}"),
            }
        }
    }

    #[test]
    fn crlf_quotes_blank_lines_and_missing_final_newline() {
        assert_eq!(
            rows("a,b\r\n\"x,1\",\"say \"\"hi\"\"\"\r\n\r\nlast,"),
            vec![
                vec!["a", "b"],
                vec!["x,1", "say \"hi\""],
                vec![""],
                vec!["last", ""]
            ]
        );
        assert_eq!(rows("a\nb\n"), vec![vec!["a"], vec!["b"]]);
        assert_eq!(rows(""), Vec::<Vec<String>>::new());
        assert_eq!(rows("\"multi\nline\",z"), vec![vec!["multi\nline", "z"]]);
        assert!(parse_rfc4180_str("a,\"open\nrow").is_err());
    }

    #[test]
    fn quotes_in_the_middle_of_a_field_are_kept_as_data() {
        // what the tolerant reader does with the two RFC violations (no character is lost but the structural quotes)
        assert_eq!(rows("a\"b,c"), vec![vec!["a\"b", "c"]]);
        assert_eq!(rows("5\" pipe"), vec![vec!["5\" pipe"]]);
        assert_eq!(rows("\"x\" ,y"), vec![vec!["x ", "y"]]);
        assert_eq!(rows("\"x\"y,z"), vec![vec!["xy", "z"]]);
        assert_eq!(rows("\"x\" \"y\",z"), vec![vec!["x \"y\"", "z"]]);
        assert_eq!(rows(" \"x\",z"), vec![vec![" \"x\"", "z"]]);
    }

    #[test]
    fn a_leading_byte_order_mark_is_not_part_of_the_first_header() {
        assert_eq!(rows("\u{feff}id,x\n1,2"), vec![vec!["id", "x"], vec!["1", "2"]]);
        let csv = "\u{feff}id,name_en,name_it,neutral_formula,monoisotopic_mass,mz_mh,mz_mna,mz_mnh4,smiles,inchikey,inchikey_connectivity,cas,pubchem_cid,effect_class,chemical_class,role,psychoactive,synonyms\r\nx,X,X,CH4,16.0313,17.0386,39.0206,34.0652,C,,,,,,,,no,\r\n";
        let e = parse_target_csv(csv.as_bytes()).unwrap();
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].id, "x");
    }
}
