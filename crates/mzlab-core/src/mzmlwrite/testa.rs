//! The header of a `<spectrum>` up to its arrays (`spectrum_head`), which `MzmlWriter` writes and `scantable` reads back.

use super::*;

/// Total ion current and base peak (m/z, intensity) summed from the points; the `precursori` build takes the maker's values where the reader gives them.
fn totals(sp: &Spectrum, extra: &SpectrumExtra) -> (f64, Option<(f64, f64)>) {
    let tic: f64 = (0..sp.mz.len()).map(|i| sp.intensity_f64(i)).sum();
    let base = (0..sp.mz.len())
        .max_by(|&a, &b| sp.intensity_f64(a).total_cmp(&sp.intensity_f64(b)))
        .map(|b| (sp.mz[b], sp.intensity_f64(b)));
    #[cfg(feature = "precursori")]
    return (extra.tic.unwrap_or(tic), extra.base_peak.or(base));
    #[cfg(not(feature = "precursori"))]
    {
        let _ = extra;
        (tic, base)
    }
}

/// The text of a spectrum up to its arrays (`<spectrum ...>` and every header element before `<binaryDataArrayList`): what `scantable` reads.
pub(crate) fn spectrum_head(index: usize, sp: &Spectrum, extra: &SpectrumExtra) -> String {
    let (tic, base) = totals(sp, extra);
    let mut s = format!(
        "<spectrum index=\"{}\" id=\"{}\" defaultArrayLength=\"{}\">",
        index,
        esc(&sp.id),
        sp.mz.len()
    );
    s += &cv("MS:1000511", "ms level", sp.level);
    s += &if sp.level <= 1 {
        cv("MS:1000579", "MS1 spectrum", "")
    } else {
        cv("MS:1000580", "MSn spectrum", "")
    };
    match sp.polarity {
        1 => s += &cv("MS:1000130", "positive scan", ""),
        -1 => s += &cv("MS:1000129", "negative scan", ""),
        _ => {}
    }
    s += &if sp.profile {
        cv("MS:1000128", "profile spectrum", "")
    } else {
        cv("MS:1000127", "centroid spectrum", "")
    };
    if let Some((mz, int)) = base {
        s += &mz_cv("MS:1000504", "base peak m/z", mz);
        s += &unit_cv(
            "MS:1000505",
            "base peak intensity",
            int,
            "MS:1000131",
            "number of detector counts",
        );
    }
    s += &cv("MS:1000285", "total ion current", tic);
    if extra.sim {
        s += &cv("MS:1000582", "SIM spectrum", "");
    }
    if let (true, Some(&lo), Some(&hi)) = (extra.observed, sp.mz.first(), sp.mz.last()) {
        s += &mz_cv("MS:1000528", "lowest observed m/z", lo);
        s += &mz_cv("MS:1000527", "highest observed m/z", hi);
    }
    #[cfg(feature = "precursori")]
    {
        s += &params_xml(&extra.cv_params);
    }
    for (k, v) in &extra.params {
        #[cfg(feature = "sciex")]
        if let Some(acc) = k.strip_prefix("cv:") {
            s += &cv(acc, v, "");
            continue;
        }
        s += &format!("<userParam name=\"{}\" value=\"{}\"/>", esc(k), esc(v));
    }
    // The filter string is a scan-level term: the CV only allows it under <scan>.
    let filter = if sp.filter.is_empty() {
        String::new()
    } else {
        cv("MS:1000512", "filter string", esc(&sp.filter))
    };
    s += &format!("<scanList count=\"1\">{}<scan><cvParam cvRef=\"MS\" accession=\"MS:1000016\" name=\"scan start time\" value=\"{}\" unitCvRef=\"UO\" unitAccession=\"UO:0000031\" unitName=\"minute\"/>{}{}{}</scan></scanList>", cv("MS:1000795", "no combination", ""), sp.rt, filter, scan_xml(extra), window_xml(extra.window));
    #[cfg(feature = "precursori")]
    if !extra.precursors.is_empty() {
        return s + &precursor_list(&extra.precursors);
    }
    if let Some(p) = sp.precursor {
        s += &format!("<precursorList count=\"1\"><precursor><isolationWindow>{}</isolationWindow><selectedIonList count=\"1\"><selectedIon>{}</selectedIon></selectedIonList></precursor></precursorList>", mz_cv("MS:1000827", "isolation window target m/z", p), mz_cv("MS:1000744", "selected ion m/z", p));
    }
    s
}
