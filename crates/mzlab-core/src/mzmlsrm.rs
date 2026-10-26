//! Dwell time of an SRM chromatogram id, which the mzML writer turns into the `MS_dwell_time` userParam that `mzmlhead` reads back.

/// The dwell time (ms) an SRM chromatogram id spells (`dwell=...`).
pub fn id_dwell(id: &str) -> Option<f64> {
    id.split_whitespace()
        .find_map(|w| w.strip_prefix("dwell=")?.parse().ok())
}
