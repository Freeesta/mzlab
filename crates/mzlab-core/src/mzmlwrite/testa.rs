//! The header of a `<spectrum>` up to its arrays (`spectrum_head`), which `MzmlWriter` writes and `scantable` reads back.

use super::*;

/// What the reader already knows of the totals: the maker's TIC and base peak (m/z, intensity), by feature (`precursori` both, `estesi` the TIC).
#[allow(unused_variables)]
fn known(extra: &SpectrumExtra) -> (Option<f64>, Option<(f64, f64)>) {
    #[cfg(feature = "precursori")]
    return (extra.tic, extra.base_peak);
    #[cfg(all(feature = "estesi", not(feature = "precursori")))]
    return (extra.tic, None);
    #[cfg(not(any(feature = "precursori", feature = "estesi")))]
    (None, None)
}

/// Total ion current and base peak (m/z, intensity): the maker's values where the reader gives them, else taken from the points in one pass over them
/// (only for the one that is missing). On equal intensities the base peak is the last of them.
fn totals(sp: &Spectrum, extra: &SpectrumExtra) -> (f64, Option<(f64, f64)>) {
    let (tic, base) = known(extra);
    if let (Some(t), Some(b)) = (tic, base) {
        return (t, Some(b));
    }
    let (want_tic, want_base) = (tic.is_none(), base.is_none());
    let (mut sum, mut best): (f64, Option<(usize, f64)>) = (0.0, None);
    let mut at = |i: usize, v: f64| {
        if want_tic {
            sum += v;
        }
        if want_base && best.is_none_or(|(_, b)| v.total_cmp(&b).is_ge()) {
            best = Some((i, v));
        }
    };
    match &sp.wide {
        Some(w) => (0..sp.mz.len()).for_each(|i| at(i, w.get(i).copied().unwrap_or(0.0))),
        None => {
            (0..sp.mz.len()).for_each(|i| at(i, sp.intensity.get(i).map_or(0.0, |&x| f64::from(x))))
        }
    }
    (
        tic.unwrap_or(sum),
        base.or(best.map(|(i, v)| (sp.mz[i], v))),
    )
}

/// Room for the header of `sp`: the fixed elements, the filter and about 130 bytes for each parameter the reader gave.
fn capacita_testa(sp: &Spectrum, extra: &SpectrumExtra) -> usize {
    #[cfg(feature = "precursori")]
    let n = extra.params.len()
        + extra.scan_params.len()
        + extra.cv_params.len()
        + 2 * extra.precursors.len();
    #[cfg(not(feature = "precursori"))]
    let n = extra.params.len();
    2048 + sp.filter.len() + 130 * n
}

/// The text of a spectrum up to its arrays (`<spectrum ...>` and every header element before `<binaryDataArrayList`): what `scantable` reads.
pub(crate) fn spectrum_head(index: usize, sp: &Spectrum, extra: &SpectrumExtra) -> String {
    // a header is a few KB: one allocation instead of the doublings of a string that starts empty
    let mut s = String::with_capacity(capacita_testa(sp, extra));
    spectrum_head_into(&mut s, index, sp, extra);
    s
}

/// `spectrum_head` appended to `s`.
pub(crate) fn spectrum_head_into(
    s: &mut String,
    index: usize,
    sp: &Spectrum,
    extra: &SpectrumExtra,
) {
    use std::fmt::Write;
    let (tic, base) = totals(sp, extra);
    s.reserve(capacita_testa(sp, extra));
    s.push_str("<spectrum index=\"");
    let _ = write!(s, "{index}\" id=\"");
    esc_into(s, &sp.id);
    let _ = write!(s, "\" defaultArrayLength=\"{}\">", sp.mz.len());
    cv_into(s, "MS:1000511", "ms level", sp.level);
    if sp.level <= 1 {
        cv_into(s, "MS:1000579", "MS1 spectrum", "");
    } else {
        cv_into(s, "MS:1000580", "MSn spectrum", "");
    }
    match sp.polarity {
        1 => cv_into(s, "MS:1000130", "positive scan", ""),
        -1 => cv_into(s, "MS:1000129", "negative scan", ""),
        _ => {}
    }
    if sp.profile {
        cv_into(s, "MS:1000128", "profile spectrum", "");
    } else {
        cv_into(s, "MS:1000127", "centroid spectrum", "");
    }
    if let Some((mz, int)) = base {
        mz_cv_into(s, "MS:1000504", "base peak m/z", mz);
        unit_cv_into(
            s,
            "MS:1000505",
            "base peak intensity",
            int,
            "MS:1000131",
            "number of detector counts",
        );
    }
    cv_into(s, "MS:1000285", "total ion current", tic);
    if extra.sim {
        cv_into(s, "MS:1000582", "SIM spectrum", "");
    }
    if let (true, Some(&lo), Some(&hi)) = (extra.observed, sp.mz.first(), sp.mz.last()) {
        mz_cv_into(s, "MS:1000528", "lowest observed m/z", lo);
        mz_cv_into(s, "MS:1000527", "highest observed m/z", hi);
    }
    #[cfg(feature = "precursori")]
    params_into(s, &extra.cv_params);
    for (k, v) in &extra.params {
        #[cfg(feature = "sciex")]
        if let Some(acc) = k.strip_prefix("cv:") {
            // the accession and the term come from the file: escaped like the other texts
            s.push_str("<cvParam cvRef=\"MS\" accession=\"");
            esc_into(s, acc);
            s.push_str("\" name=\"");
            esc_into(s, v);
            s.push_str("\" value=\"\"/>");
            continue;
        }
        s.push_str("<userParam name=\"");
        esc_into(s, k);
        s.push_str("\" value=\"");
        esc_into(s, v);
        s.push_str("\"/>");
    }
    s.push_str("<scanList count=\"1\">");
    cv_into(s, "MS:1000795", "no combination", "");
    let _ = write!(s, "<scan><cvParam cvRef=\"MS\" accession=\"MS:1000016\" name=\"scan start time\" value=\"{}\" unitCvRef=\"UO\" unitAccession=\"UO:0000031\" unitName=\"minute\"/>", sp.rt);
    // The filter string is a scan-level term: the CV only allows it under <scan>.
    if !sp.filter.is_empty() {
        s.push_str(
            "<cvParam cvRef=\"MS\" accession=\"MS:1000512\" name=\"filter string\" value=\"",
        );
        esc_into(s, &sp.filter);
        s.push_str("\"/>");
    }
    scan_into(s, extra);
    window_into(s, extra.window);
    s.push_str("</scan></scanList>");
    #[cfg(feature = "precursori")]
    if !extra.precursors.is_empty() {
        precursor_list_into(s, &extra.precursors);
        return;
    }
    if let Some(p) = sp.precursor {
        plain_precursor_into(s, p, extra);
    }
}

/// The `<precursorList>` of a spectrum whose reader gave only the selected m/z: the schema wants an `<activation>`, which is collision-induced dissociation
/// when the file does not say otherwise. The readers that know a collision energy and an isolation half-width (`SpectrumExtra::ce`, `iso_half`) add them;
/// the builds that carry no such reader (the ones under a size limit) leave them out.
fn plain_precursor_into(s: &mut String, p: f64, extra: &SpectrumExtra) {
    s.push_str("<precursorList count=\"1\"><precursor><isolationWindow>");
    mz_cv_into(s, "MS:1000827", "isolation window target m/z", p);
    #[cfg(any(feature = "sciex", feature = "precursori"))]
    if let Some(h) = extra.iso_half {
        mz_cv_into(s, "MS:1000828", "isolation window lower offset", h);
        mz_cv_into(s, "MS:1000829", "isolation window upper offset", h);
    }
    s.push_str("</isolationWindow><selectedIonList count=\"1\"><selectedIon>");
    mz_cv_into(s, "MS:1000744", "selected ion m/z", p);
    s.push_str("</selectedIon></selectedIonList><activation><cvParam cvRef=\"MS\" accession=\"MS:1000133\" name=\"collision-induced dissociation\" value=\"\"/>");
    #[cfg(any(feature = "sciex", feature = "precursori"))]
    if let Some(ce) = extra.ce {
        use std::fmt::Write;
        let _ = write!(s, "<cvParam cvRef=\"MS\" accession=\"MS:1000045\" name=\"collision energy\" value=\"{ce}\" unitCvRef=\"UO\" unitAccession=\"UO:0000266\" unitName=\"electronvolt\"/>");
    }
    #[cfg(not(any(feature = "sciex", feature = "precursori")))]
    let _ = extra;
    s.push_str("</activation></precursor></precursorList>");
}
