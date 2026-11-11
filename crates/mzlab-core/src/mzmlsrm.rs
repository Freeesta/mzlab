//! Dwell time of an SRM or SIM chromatogram id (ms), which the mzML writer turns into the `MS_dwell_time` userParam (seconds, as msconvert writes it)
//! that `mzmlhead` reads back.

/// The dwell time (ms) an SRM chromatogram id spells (`dwell=...`).
pub fn id_dwell(id: &str) -> Option<f64> {
    id.split_whitespace()
        .find_map(|w| w.strip_prefix("dwell=")?.parse().ok())
}

/// The `MS_dwell_time` userParam of the dwell an id spells, in seconds (rounded off the float noise of the division), empty when the id has none.
pub fn dwell_param(id: &str) -> String {
    id_dwell(id).map_or(String::new(), |d| {
        format!(
            "<userParam name=\"MS_dwell_time\" value=\"{}\"/>",
            crate::json::round(d / 1000.0, 7)
        )
    })
}
