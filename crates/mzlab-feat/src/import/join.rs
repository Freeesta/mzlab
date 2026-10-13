//! Recognition of the files by their columns and `build`: from the files of one table (the main table + MS2 + annotations + network) to ONE `Table`.

use super::delim::txt;
use super::model::{key_of, Table};
use super::{gnps, mgf, msdial, mzmine, ImportError};

/// A file of the table: its name, its text and the SHA-256 the page computed (kept in the table).
pub struct InFile {
    pub name: String,
    pub text: String,
    pub sha256: String,
}

/// `(^|sep)"?a"?sep"?b"?sep` on a lower-case line: the names in a row, each optionally quoted; `end_ok` lets the last one end the line.
fn fields(l: &str, sep: char, names: &[&str], quoted: bool, end_ok: bool) -> bool {
    let b: Vec<char> = l.chars().collect();
    let name_at = |mut p: usize| -> Option<usize> {
        for (k, n) in names.iter().enumerate() {
            if quoted && b.get(p) == Some(&'"') {
                p += 1;
            }
            let nc: Vec<char> = n.chars().collect();
            if b.get(p..p + nc.len())? != nc.as_slice() {
                return None;
            }
            p += nc.len();
            if quoted && b.get(p) == Some(&'"') {
                p += 1;
            }
            if b.get(p) == Some(&sep) {
                p += 1;
            } else if !(end_ok && k + 1 == names.len() && p == b.len()) {
                return None;
            }
        }
        Some(p)
    };
    (0..=b.len()).any(|p| (p == 0 || b[p - 1] == sep) && name_at(p).is_some())
}

/// `^"?#scan#"?[\t,]` on a lower-case line.
fn scan_header(l: &str) -> bool {
    let r = l.strip_prefix('"').unwrap_or(l);
    r.strip_prefix("#scan#")
        .is_some_and(|r| r.strip_prefix('"').unwrap_or(r).starts_with(['\t', ',']))
}

/// The kind of a file from its name and the beginning of its text: «mgf», «msdial», «gnps-library», «gnps-network», «mzmine-edges», «mzmine-quant», «mzmine-annotations»,
/// «mzmine-full» or «» (not recognised).
pub fn detect(name: &str, head: &str) -> &'static str {
    let head = head.strip_prefix('\u{feff}').unwrap_or(head);
    let first: Vec<&str> = head
        .split('\n')
        .take(40)
        .map(|l| l.strip_suffix('\r').unwrap_or(l))
        .collect();
    if head.to_lowercase().contains("begin ions") || name.to_lowercase().ends_with(".mgf") {
        return "mgf";
    }
    let line = first
        .iter()
        .find(|l| !txt(l).is_empty())
        .copied()
        .unwrap_or("");
    let l = line.to_lowercase();
    if first
        .iter()
        .any(|x| fields(&x.to_lowercase(), '\t', &["alignment id"], false, true))
    {
        return "msdial";
    }
    if scan_header(&l) && l.contains("mqscore") {
        "gnps-library"
    } else if l.contains("cluster index") && l.contains("component") {
        "gnps-network"
    } else if fields(&l, ',', &["id1", "id2"], true, false) && l.contains("edgetype") {
        "mzmine-edges"
    } else if l.contains("row id") && l.contains("row m/z") {
        "mzmine-quant"
    } else if fields(&l, ',', &["id"], true, false)
        && l.contains("compound_name")
        && fields(&l, ',', &["method"], true, true)
    {
        "mzmine-annotations"
    } else if ["id", "mz", "rt"]
        .iter()
        .all(|n| fields(&l, ',', &[n], true, false))
    {
        "mzmine-full"
    } else {
        ""
    }
}

fn side(t: &mut Table, kind: &str, text: &str) -> Result<bool, ImportError> {
    match kind {
        "mgf" => drop(mgf::attach_ms2(t, &mgf::read_mgf(text))),
        "mzmine-annotations" => mzmine::merge_annotations(t, text)?,
        "mzmine-edges" => drop(mzmine::read_edges(t, text)?),
        "gnps-library" => gnps::read_gnps_library(t, text)?,
        "gnps-network" => drop(gnps::read_gnps_network(t, text)?),
        _ => return Ok(false),
    }
    Ok(true)
}

/// ONE table from a list of files; `pol` is 1, -1 or 0 (not known), `max_features` > 0 limits the main table.
pub fn build(files: &[InFile], pol: i32, max_features: usize) -> Result<Table, ImportError> {
    let kinds: Vec<&'static str> = files
        .iter()
        .map(|f| detect(&f.name, &f.text.chars().take(20000).collect::<String>()))
        .collect();
    let find = |k: &str| kinds.iter().position(|x| *x == k);
    let Some(mi) = find("mzmine-full")
        .or_else(|| find("mzmine-quant"))
        .or_else(|| find("msdial"))
    else {
        return Err(ImportError::new("noTable", "No feature table found: expected an mzMine table (id, mz, rt), an FBMN quant table or an MS-DIAL alignment"));
    };
    let main = &files[mi];
    let mut t = match kinds[mi] {
        "msdial" => msdial::read_msdial(&main.text, &main.name, pol)?,
        "mzmine-quant" => mzmine::read_quant(&main.text, &main.name, pol)?,
        _ => mzmine::read_full(&main.text, &main.name, pol, max_features)?,
    };
    t.sha256 = main.sha256.clone();
    t.used.push(main.name.clone());
    for (i, f) in files.iter().enumerate().filter(|(i, _)| *i != mi) {
        match side(&mut t, kinds[i], &f.text) {
            Ok(true) => t.used.push(f.name.clone()),
            Ok(false) => t.ignored.push(f.name.clone()),
            Err(e) => {
                t.warn(format!("{}: {}", f.name, e.msg));
                t.ignored.push(f.name.clone());
            }
        }
    }
    if kinds[mi] == "mzmine-full" {
        mzmine::check_structures(&mut t);
    }
    if t.pol == 0 {
        t.warn("Polarity not found in the files: choose it before matching.".into());
    }
    for f in t.features.iter_mut() {
        f.key = key_of(t.pol, &f.id);
    }
    Ok(t)
}
