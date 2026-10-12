//! The routes of the scans on a file that has none (an MRM file: only transition chromatograms): the answers are empty, as `Item` of `mzlab/explore.py` gives them. The
//! ones that fail in Python (`nearest_scan`, `scaninfo`) are not answered here.

use crate::routes::Query;

/// The empty answer of `path`, `None` for the routes that are not about scans.
pub fn no_scans(path: &str, q: &Query) -> Option<String> {
    Some(match path {
        "/api/filters" => "{\"filters\":[]}".to_string(),
        "/api/scan_axis" => "{\"rt\":[],\"sid\":[]}".to_string(),
        "/api/scanlist" => format!(
            "{{\"level\":{},\"n\":0,\"sid\":[],\"no\":[],\"rt\":[],\"tic\":[],\"prec\":[],\"bpmz\":[],\"bpint\":[]}}",
            q.level()
        ),
        "/api/spectrum" => "{\"mode\":\"centroid\",\"mz\":[],\"y\":[],\"scans\":0,\"i0\":null,\"i1\":null,\"n\":0}".to_string(),
        "/api/dda" => "{\"sid\":[],\"rt\":[],\"no\":[],\"prec\":[],\"tgt\":[],\"parent\":[],\"prt\":[],\"lo\":[],\"hi\":[],\"act\":[],\"ce\":[],\"pint\":[],\"ms1\":{\"sid\":[],\"rt\":[]},\"nce\":false}".to_string(),
        _ => return None,
    })
}
