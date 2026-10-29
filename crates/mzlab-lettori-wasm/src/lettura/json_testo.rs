//! The pieces of the JSON texts of the module (`sessione.rs`, `lib.rs`), written with `mzlab_core::json::Out`: lists of strings, rows of strings, nullable strings, the samples and the points of a trace.
use mzlab_core::json::Out;
use mzlab_core::lettori::{Campione, Campo, VoceCromatogramma};
use mzlab_core::mzml::Chromatogram;
use mzlab_core::mzmlwrite::Header;

/// `[n,...]` as `Out::floats` writes them (finite numbers as Rust prints them, NaN and infinities `null`), without the temporary `String` of `Out::num` per point.
pub fn floats(o: &mut Out, a: &[f64]) {
    use std::fmt::Write;
    o.0.push('[');
    for (i, &v) in a.iter().enumerate() {
        if i > 0 {
            o.0.push(',');
        }
        if v.is_finite() {
            let _ = write!(o.0, "{v}");
        } else {
            o.0.push_str("null");
        }
    }
    o.0.push(']');
}

/// `[{id,name,scans}]`, shared with the `probe` answer of `lib.rs`.
pub fn campioni_json(o: &mut Out, campioni: &[Campione]) {
    o.raw("[");
    for (i, c) in campioni.iter().enumerate() {
        o.raw(if i > 0 { ",{\"id\":" } else { "{\"id\":" })
            .int(i64::from(c.id))
            .raw(",\"name\":")
            .str(&c.nome)
            .raw(",\"scans\":")
            .int(c.scansioni as i64)
            .raw("}");
    }
    o.raw("]");
}

pub fn opt_str(o: &mut Out, v: Option<&str>) {
    match v {
        Some(t) => o.str(t),
        None => o.raw("null"),
    };
}

/// `["a","b",...]`.
pub fn strs<'a>(o: &mut Out, v: impl IntoIterator<Item = &'a str>) {
    o.raw("[");
    for (i, t) in v.into_iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(t);
    }
    o.raw("]");
}

/// `[[a,b],...]` (each row from `row`, which writes the strings of one tuple).
pub fn rows<T>(o: &mut Out, v: &[T], row: impl Fn(&T) -> Vec<&str>) {
    o.raw("[");
    for (i, t) in v.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" });
        strs(o, row(t));
    }
    o.raw("]");
}

/// `[{name,value,unit,cv}]`.
pub fn intestazione_json(campi: &[Campo]) -> String {
    let mut o = Out::default();
    o.raw("[");
    for (i, f) in campi.iter().enumerate() {
        o.raw(if i > 0 { ",{\"name\":" } else { "{\"name\":" })
            .str(&f.nome)
            .raw(",\"value\":")
            .str(&f.valore)
            .raw(",\"unit\":");
        opt_str(&mut o, f.unita.as_deref());
        o.raw(",\"cv\":");
        opt_str(&mut o, f.cv.as_deref());
        o.raw("}");
    }
    o.raw("]");
    o.0
}

/// `[{id,kind,unit}]`.
pub fn cromatogrammi_json(v: &[VoceCromatogramma]) -> String {
    let mut o = Out::default();
    o.raw("[");
    for (i, x) in v.iter().enumerate() {
        o.raw(if i > 0 { ",{\"id\":" } else { "{\"id\":" })
            .str(&x.id)
            .raw(",\"kind\":")
            .str(&format!("{:?}", x.tipo))
            .raw(",\"unit\":")
            .str(&x.unita)
            .raw("}");
    }
    o.raw("]");
    o.0
}

/// `{id,kind,q1,q3,unit,time,intensity}`.
pub fn cromatogramma_json(x: &Chromatogram) -> String {
    let mut o = Out(String::with_capacity(
        96 + 20 * (x.time.len() + x.intensity.len()),
    ));
    o.raw("{\"id\":")
        .str(&x.id)
        .raw(",\"kind\":")
        .str(&format!("{:?}", x.kind))
        .raw(",\"q1\":")
        .opt(x.q1)
        .raw(",\"q3\":")
        .opt(x.q3)
        .raw(",\"unit\":")
        .str(&x.unit)
        .raw(",\"time\":");
    floats(&mut o, &x.time);
    o.raw(",\"intensity\":");
    floats(&mut o, &x.intensity);
    o.raw("}");
    o.0
}

/// The `Header` of the mzML as the object the page reads (`Sessioni::testata`).
pub fn testata_json(t: &Header) -> String {
    let mut o = Out::default();
    o.raw("{\"run\":").str(&t.run).raw(",\"sources\":");
    strs(&mut o, t.sources.iter().map(String::as_str));
    o.raw(",\"instrument\":");
    par_opt(&mut o, &t.instrument);
    o.raw(",\"components\":");
    rows(&mut o, &t.components, |(a, b, c)| vec![a, b, c]);
    o.raw(",\"software\":");
    strs(&mut o, [t.software.0.as_str(), t.software.1.as_str()]);
    o.raw(",\"acquired\":");
    opt_str(&mut o, t.acquired.as_deref());
    o.raw(",\"serial\":");
    opt_str(&mut o, t.serial.as_deref());
    o.raw(",\"source_sha1\":");
    strs(&mut o, t.source_sha1.iter().map(String::as_str));
    o.raw(",\"source_cv\":");
    rows(&mut o, &t.source_cv, |(a, b)| vec![a, b]);
    o.raw(",\"sample\":");
    par_opt(&mut o, &t.sample);
    o.raw(",\"run_params\":");
    rows(&mut o, &t.run_params, |(a, b)| vec![a, b]);
    o.raw(",\"content\":");
    rows(&mut o, &t.content, |(a, b)| vec![a, b]);
    o.raw(",\"extra_software\":");
    rows(&mut o, &t.extra_software, |(a, b, c)| vec![a, b, c]);
    o.raw(",\"centroided\":")
        .raw(if t.centroided { "true" } else { "false" })
        .raw("}");
    o.0
}

/// `[a,b]` or `null`.
fn par_opt(o: &mut Out, v: &Option<(String, String)>) {
    match v {
        Some((a, b)) => strs(o, [a.as_str(), b.as_str()]),
        None => {
            o.raw("null");
        }
    }
}
