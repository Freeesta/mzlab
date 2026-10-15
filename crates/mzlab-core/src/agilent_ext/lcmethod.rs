//! The method of an Agilent LC/MS run (feature `estesi`): `AcqData/AcqMethod.xml` is a report that holds two XML documents as escaped text, `RCDevicesXml` (one
//! `Section` per parameter of each LC module: sampler, pump, column compartment, detector) and `SCICDevicesXml` (the mass spectrometer: `SectionInfo` rows that
//! are repeaters, sections, tables, columns and parameters in document order). Both are read as lists of rows; `lcdoc` turns them into the `metodo/1` sections.
//! Nothing is guessed: a table is what its column names say, and a cell that is not a number stays a text in `campi`.

use std::collections::BTreeMap;

/// A parameter of an LC module.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Rc {
    /// `ModuleDisplayName` ("Binary Pump") and `DeviceType`.
    pub module: String,
    pub kind: String,
    /// The section the parameter belongs to (the parent section when it has one).
    pub section: String,
    pub table: Option<String>,
    pub row: Option<usize>,
    pub name: String,
    pub value: Option<String>,
    pub unit: Option<String>,
}

/// A scalar parameter of the mass spectrometer, with the time segment (0: before the first) and the section it follows.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MsParam {
    pub seg: u32,
    pub section: String,
    pub name: String,
    pub value: Option<String>,
}

/// A table of the mass spectrometer: the column names and the rows of cells.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MsTable {
    pub seg: u32,
    pub name: String,
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Option<String>>>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct LcMethod {
    pub name: Option<String>,
    pub rc: Vec<Rc>,
    pub ms: Vec<MsParam>,
    pub tables: Vec<MsTable>,
    /// (segment, mode) from the sections «Acquisition Mode X» of a Q-TOF method.
    pub modes: Vec<(u32, String)>,
}

fn between<'a>(s: &'a str, open: &str, close: &str) -> Option<&'a str> {
    let a = s.find(open)? + open.len();
    Some(&s[a..a + s[a..].find(close)?])
}

/// `&lt; &gt; &amp; &quot; &apos; &#N;` replaced; anything else is left as it is.
pub fn unescape(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(a) = rest.find('&') {
        out.push_str(&rest[..a]);
        rest = &rest[a..];
        let Some(end) = rest.find(';').filter(|e| *e <= 8) else {
            out.push('&');
            rest = &rest[1..];
            continue;
        };
        let ent = &rest[1..end];
        let ch = match ent {
            "lt" => Some('<'),
            "gt" => Some('>'),
            "amp" => Some('&'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => ent
                .strip_prefix('#')
                .and_then(|n| {
                    n.strip_prefix('x')
                        .map_or_else(|| n.parse().ok(), |h| u32::from_str_radix(h, 16).ok())
                })
                .and_then(char::from_u32),
        };
        match ch {
            Some(c) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out + rest
}

/// The text of the element `name` inside `block` (None when absent or empty).
fn tag(block: &str, name: &str) -> Option<String> {
    let t = between(block, &format!("<{name}>"), &format!("</{name}>"))?;
    let t = unescape(t);
    let t = t.trim();
    (!t.is_empty()).then(|| t.to_string())
}

fn blocks<'a>(xml: &'a str, name: &str) -> Vec<&'a str> {
    let (open, close) = (format!("<{name}>"), format!("</{name}>"));
    xml.split(open.as_str())
        .skip(1)
        .map(|p| p.split(close.as_str()).next().unwrap_or(""))
        .collect()
}

fn parse_rc(xml: &str) -> Vec<Rc> {
    blocks(xml, "Section")
        .into_iter()
        .filter_map(|b| {
            let name = tag(b, "ParameterName")?;
            let own = tag(b, "SectionName");
            Some(Rc {
                module: tag(b, "ModuleDisplayName").unwrap_or_default(),
                kind: tag(b, "DeviceType").unwrap_or_default(),
                section: tag(b, "ParentSectionName").or(own).unwrap_or_default(),
                table: tag(b, "TableName"),
                row: tag(b, "RowIndex").and_then(|r| r.parse().ok()),
                name,
                value: tag(b, "ParameterValue"),
                unit: tag(b, "ParameterUnit"),
            })
        })
        .collect()
}

fn parse_ms(xml: &str, m: &mut LcMethod) {
    let (mut seg, mut ctx, mut cur) = (0u32, String::new(), None::<usize>);
    // cells by table: (row, column) → value
    let mut cells: Vec<BTreeMap<(usize, usize), Option<String>>> = Vec::new();
    for b in blocks(xml, "SectionInfo") {
        let name = tag(b, "Name");
        match tag(b, "SectionType").as_deref() {
            Some("Repeater" | "Section") => {
                cur = None;
                if let Some(n) = name {
                    if let Some(k) = n
                        .strip_prefix("Time Segment ")
                        .and_then(|v| v.trim().parse().ok())
                    {
                        seg = k;
                    }
                    if let Some(mode) = n.strip_prefix("Acquisition Mode ") {
                        m.modes.push((seg, mode.trim().to_string()));
                    }
                    ctx = n;
                }
            }
            Some("Table") => {
                m.tables.push(MsTable {
                    seg,
                    name: name.unwrap_or_default(),
                    ..Default::default()
                });
                cells.push(BTreeMap::new());
                cur = Some(m.tables.len() - 1);
            }
            Some("Column") => {
                let (Some(t), Some(row), Some(col)) = (
                    cur,
                    tag(b, "RowIndex").and_then(|v| v.parse::<usize>().ok()),
                    tag(b, "ColumnIndex").and_then(|v| v.parse::<usize>().ok()),
                ) else {
                    continue;
                };
                // a table of thousands of cells in a damaged file is not worth a quadratic grid
                if row > 100_000 || col > 256 {
                    continue;
                }
                let table = &mut m.tables[t];
                if table.columns.len() <= col {
                    table.columns.resize(col + 1, String::new());
                }
                if table.columns[col].is_empty() {
                    table.columns[col] = name.unwrap_or_default();
                }
                cells[t].insert((row, col), tag(b, "Value"));
            }
            Some("Parameter") => m.ms.push(MsParam {
                seg,
                section: ctx.clone(),
                name: name.unwrap_or_default(),
                value: tag(b, "Value"),
            }),
            _ => {}
        }
    }
    for (t, c) in m.tables.iter_mut().zip(cells) {
        let rows = c.keys().map(|k| k.0 + 1).max().unwrap_or(0);
        t.rows = vec![vec![None; t.columns.len()]; rows];
        for ((r, k), v) in c {
            t.rows[r][k] = v;
        }
    }
}

impl LcMethod {
    /// Reads `AcqMethod.xml`; None when the text has neither of the two documents.
    pub fn parse(text: &str) -> Option<LcMethod> {
        let mut m = LcMethod {
            name: tag(text, "MethodName"),
            ..Default::default()
        };
        let rc = between(text, "<RCDevicesXml>", "</RCDevicesXml>").map(unescape);
        let ms = between(text, "<SCICDevicesXml>", "</SCICDevicesXml>").map(unescape);
        if rc.is_none() && ms.is_none() {
            return None;
        }
        if let Some(x) = rc {
            m.rc = parse_rc(&x);
        }
        if let Some(x) = ms {
            parse_ms(&x, &mut m);
        }
        Some(m)
    }

    /// A table by name, the first of the segment `seg` (any segment when None).
    pub fn table(&self, name: &str, seg: Option<u32>) -> Option<&MsTable> {
        self.tables
            .iter()
            .find(|t| t.name == name && seg.is_none_or(|s| t.seg == s))
    }

    /// The first scalar parameter called `name`.
    pub fn param(&self, name: &str) -> Option<&str> {
        self.ms
            .iter()
            .find(|p| p.name == name)
            .and_then(|p| p.value.as_deref())
    }

    /// The first LC parameter of a module kind called `name`, outside any table.
    pub fn rc_param(&self, kind: &str, name: &str) -> Option<&Rc> {
        self.rc
            .iter()
            .find(|p| p.kind == kind && p.name == name && p.table.is_none() && p.value.is_some())
    }

    /// The segments of the method (the `Time Segments` table says how many; a method without it has the segments its parameters name).
    pub fn segments(&self) -> Vec<u32> {
        let mut v: Vec<u32> = self
            .tables
            .iter()
            .map(|t| t.seg)
            .chain(self.ms.iter().map(|p| p.seg))
            .filter(|s| *s > 0)
            .collect();
        v.sort_unstable();
        v.dedup();
        v
    }
}

impl MsTable {
    pub fn column(&self, name: &str) -> Option<usize> {
        self.columns.iter().position(|c| c == name)
    }

    pub fn cell(&self, row: usize, name: &str) -> Option<&str> {
        self.rows.get(row)?.get(self.column(name)?)?.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unescape_entities() {
        assert_eq!(
            unescape("a &lt;b&gt; &amp;&#228;&#xE4; &bogus; &"),
            "a <b> &ää &bogus; &"
        );
    }

    #[test]
    fn the_two_documents_become_rows() {
        let rc = "<DSRdlReport><Section><ModuleDisplayName>Binary Pump</ModuleDisplayName><DeviceType>Binary Pump</DeviceType><ParentSectionName>Stop Time</ParentSectionName><ParameterName>Stoptime</ParameterName><ParameterValue>10.00</ParameterValue><ParameterUnit>min</ParameterUnit></Section></DSRdlReport>";
        let ms = "<NewDataSet><SectionInfo><SectionType>Repeater</SectionType><Name>Time Segment 1</Name></SectionInfo><SectionInfo><SectionType>Table</SectionType><Name>Scan Segments</Name></SectionInfo><SectionInfo><SectionType>Column</SectionType><Name>Prec Ion</Name><RowIndex>0</RowIndex><ColumnIndex>0</ColumnIndex><Value>399.1</Value></SectionInfo><SectionInfo><SectionType>Column</SectionType><Name>Prod Ion</Name><RowIndex>1</RowIndex><ColumnIndex>1</ColumnIndex><Value>250.2</Value></SectionInfo><SectionInfo><SectionType>Parameter</SectionType><Name>Ion Source</Name><Value>ESI</Value></SectionInfo></NewDataSet>";
        let esc = |s: &str| {
            s.replace('&', "&amp;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
        };
        let text = format!(
            "<DataFileMethodReport><MethodReport><MethodName>a.m</MethodName><RCDevicesXml>{}</RCDevicesXml><SCICDevicesXml>{}</SCICDevicesXml></MethodReport></DataFileMethodReport>",
            esc(rc),
            esc(ms)
        );
        let m = LcMethod::parse(&text).unwrap();
        assert_eq!(m.name.as_deref(), Some("a.m"));
        assert_eq!(m.rc[0].name, "Stoptime");
        assert_eq!(m.rc[0].section, "Stop Time");
        let t = m.table("Scan Segments", Some(1)).unwrap();
        assert_eq!(t.rows.len(), 2);
        assert_eq!(t.cell(0, "Prec Ion"), Some("399.1"));
        assert_eq!(t.cell(1, "Prod Ion"), Some("250.2"));
        assert_eq!(t.cell(0, "Prod Ion"), None);
        assert_eq!(m.param("Ion Source"), Some("ESI"));
        assert!(LcMethod::parse("<x/>").is_none());
    }
}
