//! The text of `massacc/1` (`docs/schemi/massacc-1.json`): per file the numbers, the points of the two charts, the standards and the warnings; over the files the offsets and the
//! standards' spread; and the advice on the tolerance. Numbers only, no verdict. Called by `massacc::run`.

use mzlab_core::json::{round, Out};

use super::analyse::FileResult;
use super::refs::RefIon;
use super::{area_warning, IstdBatch, Params};

fn opt(o: &mut Out, v: Option<f64>, nd: usize) {
    o.opt(v.map(|x| round(x, nd)));
}

fn strings(o: &mut Out, v: &[&str]) {
    o.raw("[");
    for (i, k) in v.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(k);
    }
    o.raw("]");
}

fn params(o: &mut Out, p: &Params) {
    let n = |o: &mut Out, k: &str, v: f64| {
        o.raw(",\"").raw(k).raw("\":").num(v);
    };
    o.raw("{\"robust\":")
        .str(p.robust.name())
        .raw(",\"model\":")
        .str(p.model.name());
    n(o, "search_ppm", p.search_ppm);
    n(o, "min_intensity", p.min_intensity);
    n(o, "min_refs", p.min_refs as f64);
    n(o, "warn_offset_ppm", p.warn_offset_ppm);
    n(o, "warn_spread_ppm", p.warn_spread_ppm);
    n(o, "warn_is_rt_min", p.warn_is_rt_min);
    n(o, "warn_is_area_rsd", p.warn_is_area_rsd);
    o.raw("}");
}

/// The points of the charts of one file.
fn points(o: &mut Out, f: &FileResult, refs: &[RefIon]) {
    o.raw(",\"points\":[");
    for (i, pt) in f.points.iter().enumerate() {
        let r = &refs[pt.r];
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"name\":")
            .str(&r.name);
        o.raw(",\"role\":")
            .str(r.role.name())
            .raw(",\"source\":")
            .str(if r.background { "background" } else { "user" });
        o.raw(",\"mz\":")
            .num(round(r.mz, 6))
            .raw(",\"rt\":")
            .num(round(pt.rt, 4));
        o.raw(",\"err_ppm\":")
            .num(round(pt.err_ppm, 4))
            .raw(",\"intensity\":")
            .num(round(pt.intensity, 0));
        o.raw(",\"outlier\":")
            .raw(if pt.outlier { "true" } else { "false" })
            .raw("}");
    }
}

/// The standards found in one file.
fn standards(o: &mut Out, f: &FileResult, refs: &[RefIon]) {
    o.raw("],\"istd\":[");
    for (i, s) in f.istds.iter().enumerate() {
        let r = &refs[s.r];
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"name\":")
            .str(&r.name)
            .raw(",\"role\":")
            .str(r.role.name());
        o.raw(",\"rt\":")
            .num(round(s.rt, 4))
            .raw(",\"declared_rt\":");
        opt(o, r.rt, 4);
        o.raw(",\"rt_shift\":");
        opt(o, r.rt.map(|d| s.rt - d), 4);
        o.raw(",\"area\":").num(round(s.area, 0)).raw(",\"fwhm\":");
        opt(o, s.fwhm, 4);
        o.raw("}");
    }
}

fn file(o: &mut Out, f: &FileResult, refs: &[RefIon], corrected: bool) {
    o.raw("{\"file\":")
        .str(&f.name)
        .raw(",\"polarity\":")
        .int(i64::from(f.pol));
    o.raw(",\"istd_file\":")
        .raw(if f.istd_file { "true" } else { "false" });
    o.raw(",\"ms1_scans\":")
        .int(f.n_scans as i64)
        .raw(",\"searched\":")
        .int(f.searched as i64);
    o.raw(",\"n_refs\":").int(f.points.len() as i64);
    o.raw(",\"assessable\":")
        .raw(if f.assessed { "true" } else { "false" });
    o.raw(",\"offset_ppm\":");
    opt(o, f.offset, 4);
    o.raw(",\"spread_ppm\":");
    opt(o, f.spread, 4);
    o.raw(",\"residual_ppm\":");
    opt(o, f.residual, 4);
    o.raw(",\"tolerance_ppm\":");
    opt(o, f.tolerance_ppm, 0);
    o.raw(",\"model\":");
    if f.fitted {
        o.str(f.model.name()).raw(",\"coef\":{");
        for (i, (name, c)) in f.model.coef_names().iter().zip(&f.coef).enumerate() {
            o.raw(if i > 0 { "," } else { "" })
                .str(name)
                .raw(":")
                .num(round(*c, 6));
        }
        o.raw("}");
    } else {
        o.raw("null,\"coef\":null");
    }
    o.raw(",\"corrected\":").raw(if corrected && f.fitted {
        "true"
    } else {
        "false"
    });
    o.raw(",\"warnings\":");
    strings(o, &f.warnings);
    o.raw(",\"lock_mass\":");
    match &f.lock_mass {
        Some(v) => mzlab_core::json::write_value(o, v),
        None => {
            o.raw("null");
        }
    }
    points(o, f, refs);
    standards(o, f, refs);
    o.raw("]}");
}

pub(super) fn write(
    p: &Params,
    refs: &[RefIon],
    files: &[FileResult],
    batch: &[IstdBatch],
    has_table: bool,
    corrected: bool,
) -> String {
    let mut o = Out::default();
    o.raw("{\"format\":\"massacc/1\",\"params\":");
    params(&mut o, p);
    let background = refs.iter().filter(|r| r.background).count();
    o.raw(",\"references\":{\"background\":")
        .int(background as i64);
    o.raw(",\"user\":")
        .int((refs.len() - background) as i64)
        .raw("}");
    o.raw(",\"correction\":{\"model\":").str(p.model.name());
    o.raw(",\"table\":")
        .raw(if has_table { "true" } else { "false" });
    o.raw(",\"applied\":")
        .raw(if corrected { "true" } else { "false" })
        .raw("}");
    o.raw(",\"files\":[");
    for (i, f) in files.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" });
        file(&mut o, f, refs, corrected);
    }
    o.raw("],\"batch\":{\"offsets\":[");
    for (i, f) in files.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"file\":")
            .str(&f.name)
            .raw(",\"offset_ppm\":");
        opt(&mut o, f.offset, 4);
        o.raw("}");
    }
    o.raw("],\"istd_area\":[");
    for (i, b) in batch.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"name\":")
            .str(&b.name)
            .raw(",\"files\":")
            .int(b.n as i64);
        o.raw(",\"median_area\":")
            .num(round(b.median_area, 0))
            .raw(",\"rsd\":");
        opt(&mut o, b.rsd, 4);
        o.raw("}");
    }
    let tol = files
        .iter()
        .filter_map(|f| f.tolerance_ppm)
        .fold(None, |m: Option<f64>, x| Some(m.map_or(x, |y| y.max(x))));
    o.raw("],\"tolerance_ppm\":");
    opt(&mut o, tol, 0);
    let mut warnings: Vec<&str> = files
        .iter()
        .flat_map(|f| f.warnings.iter().copied())
        .collect();
    if area_warning(batch, p) {
        warnings.push("warn.qc.mass.is_area");
    }
    let warnings: Vec<&str> = super::WARNINGS
        .iter()
        .copied()
        .filter(|k| warnings.contains(k))
        .collect();
    o.raw(",\"warnings\":");
    strings(&mut o, &warnings);
    o.raw("}}");
    o.0
}
