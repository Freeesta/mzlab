//! One chromatogram of a reader as the JSON text the page reads (`{id,kind,q1,q3,unit,time,intensity}`): the readers module of the site
//! (`mzlab-lettori-wasm`, `cromatogramma`) and the app (`natregistro` of `rpc.rs`) write it with this one function, so both give the same bytes.
use mzlab_core::json::Out;
use mzlab_core::mzml::Chromatogram;

/// `[n,...]` as `Out::floats` writes them (finite numbers as Rust prints them, NaN and infinities `null`), without the temporary `String` of `Out::num` per point.
fn floats(o: &mut Out, a: &[f64]) {
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
