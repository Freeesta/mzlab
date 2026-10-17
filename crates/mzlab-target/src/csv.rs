// RFC 4180 CSV parser and target entry deserializer.
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

/// Tokenize and parse raw CSV text adhering to RFC 4180 standards.
pub fn parse_rfc4180_str(content: &str) -> Result<Vec<Vec<String>>, ParseError> {
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
        return Err(ParseError::CsvFormat {
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
    fn from_header(header: &[String]) -> Result<Self, ParseError> {
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
    row: &[String],
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
    row: &[String],
    h: &HeaderIndices,
    line: usize,
) -> Result<RowMeta, ParseError> {
    let get_str = |idx: usize, col: &'static str| -> Result<String, ParseError> {
        row.get(idx)
            .map(|s| s.trim().to_string())
            .ok_or_else(|| ParseError::MissingColumn {
                line,
                column: col.to_string(),
            })
    };

    let cas_raw = get_str(h.cas, "cas")?;
    let cas = if cas_raw.is_empty() {
        None
    } else {
        Some(cas_raw)
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
                value: cid_raw.clone(),
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
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
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

fn parse_row(row: &[String], h: &HeaderIndices, line: usize) -> Result<TargetEntry, ParseError> {
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
    let records = parse_rfc4180_str(&content)?;

    let (header, data_rows) = records.split_first().ok_or_else(|| ParseError::CsvFormat {
        line: 0,
        message: "Empty CSV input".to_string(),
    })?;

    let indices = HeaderIndices::from_header(header)?;
    let mut entries = Vec::with_capacity(data_rows.len());

    for (row_idx, row) in data_rows.iter().enumerate() {
        let line_num = row_idx + 2;
        // Skip trailing empty lines
        if row.is_empty() || (row.len() == 1 && row[0].trim().is_empty()) {
            continue;
        }
        let entry = parse_row(row, &indices, line_num)?;
        entries.push(entry);
    }

    Ok(entries)
}
