//! Collision energy and dwell time of an SRM chromatogram, the two numbers that `mzmlhead` and the mzML writer take from an id or a cvParam.

/// The dwell time (ms) an SRM chromatogram id spells (`dwell=...`).
pub fn id_dwell(id: &str) -> Option<f64> {
    id.split_whitespace()
        .find_map(|w| w.strip_prefix("dwell=")?.parse().ok())
}

/// The `value` of the first cvParam with accession `acc` of a chromatogram header, as a number (the collision energy of its `<activation>`).
pub fn cv_number(h: &str, acc: &str) -> Option<f64> {
    let t = &h[h.find(&format!("accession=\"{acc}\""))?..];
    let t = &t[..t.find("/>")?];
    let v = &t[t.find("value=\"")? + 7..];
    v[..v.find('"')?].parse().ok()
}
