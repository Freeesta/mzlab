//! The LC/MS layouts (feature `estesi`): the xsd of a QqQ or Q-TOF run nests complex types (`DataDependentScanParamType`, `SpectrumParamsType`) inside the
//! scan record; they are expanded in place, in order, as the packed record has them.

use super::*;

impl Layout {
    pub fn from_xsd_ext(xsd: &str) -> Result<Layout> {
        let clean = strip_comments(xsd);
        let mut flat = Vec::new();
        expand(&clean, "ScanRecordType", 0, &mut flat)?;
        if !flat.iter().any(|(n, _)| n == "SpectrumOffset") {
            return Err(bad("MSScan.xsd has no SpectrumParamValues".into()));
        }
        Layout::new(&flat, &[])
    }

    /// Some xsd files (the ones of the Q-TOF 6520/6530/6540 of MassHunter B.05) declare `ChromScaleFactor` as `double` while the record holds a 4-byte float:
    /// when the declared layout does not divide the records and the float one does, the float one is the layout of the file.
    pub fn fit(self, bytes: &[u8]) -> Layout {
        let start = bytes
            .get(DATA_OFFSET_AT..DATA_OFFSET_AT + 4)
            .map(|b| i32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .filter(|s| *s >= 0 && (*s as usize) <= bytes.len())
            .map_or(0, |s| s as usize);
        let rest = bytes.len().saturating_sub(start);
        if start == 0 || rest.is_multiple_of(self.record_len) {
            return self;
        }
        let swapped: Vec<(String, Kind)> = self
            .fields
            .iter()
            .map(|f| {
                let k = if f.name == "ChromScaleFactor" && f.kind == Kind::F64 {
                    Kind::F32
                } else {
                    f.kind
                };
                (f.name.clone(), k)
            })
            .collect();
        match Layout::new(&swapped, &[]) {
            Ok(l) if rest.is_multiple_of(l.record_len) => l,
            _ => self,
        }
    }
}

/// (name, type) of the elements of the complex type called `type_name` (`<xs:complexType name="…">`, not an element of that name).
fn elements_of_type(xsd: &str, type_name: &str) -> Option<Vec<(String, String)>> {
    let key = format!("<xs:complexType name=\"{type_name}\"");
    let body = &xsd[xsd.find(&key)?..];
    let body = &body[..body.find("</xs:complexType>").unwrap_or(body.len())];
    let mut out = Vec::new();
    for t in body.split("<xs:element").skip(1) {
        let tag = &t[..t.find('>').unwrap_or(t.len())];
        if let (Some(n), Some(ty)) = (attr(tag, "name"), attr(tag, "type")) {
            out.push((n, ty));
        }
    }
    Some(out)
}

/// The simple fields of `type_name`, a nested complex type replaced by its own fields (at most 4 levels).
fn expand(xsd: &str, type_name: &str, depth: u8, out: &mut Vec<(String, Kind)>) -> Result<()> {
    let els = elements_of_type(xsd, type_name)
        .ok_or_else(|| bad(format!("MSScan.xsd has no {type_name}")))?;
    for (name, ty) in els {
        match Kind::of_xsd(&ty) {
            Some(k) => out.push((name, k)),
            None if depth < 4 && elements_of_type(xsd, &ty).is_some() => {
                expand(xsd, &ty, depth + 1, out)?
            }
            None => {
                return Err(bad(format!(
                    "MSScan.xsd: field {name} has the unknown type {ty}"
                )))
            }
        }
    }
    Ok(())
}
