//! Parts of the detail of a candidate: criteria, spectrum, isotopes and the prediction.

use super::*;

pub(super) fn criteria(o: &mut Out, c: &Cand) {
    o.raw("[");
    let mut first = true;
    for (k, _, name) in KINDS.iter() {
        for p in c.ident.proofs.iter().filter(|p| p.kind == k.name()) {
            let status = match p.sign.as_str() {
                "a favore" if p.points > 0.0 => "pass",
                "contro" => "fail",
                _ => "na",
            };
            o.raw(if first {
                "{\"status\":"
            } else {
                ",{\"status\":"
            })
            .str(status)
            .raw(",\"name\":")
            .str(name);
            o.raw(",\"text\":").str(&proof_text(&[p])).raw("}");
            first = false;
        }
    }
    o.raw("]");
}

pub(super) fn spectrum(o: &mut Out, mz: &[f64], rel: &[f64], n: usize) {
    o.raw("{\"mz\":")
        .floats(mz)
        .raw(",\"rel\":")
        .floats(rel)
        .raw(",\"n_scans\":")
        .int(n as i64)
        .raw("}");
}

/// `isotopes` of `detail`: the proof of the isotope pattern as status and text.
pub(super) fn isotopes(o: &mut Out, c: &Cand) {
    let Some(p) = c.ident.proofs.iter().find(|p| p.kind == "isotopi") else {
        o.raw("null");
        return;
    };
    let st = if p.sign == "contro" {
        "inconsistent"
    } else if p.points > 0.0 {
        "consistent"
    } else {
        "not decisive"
    };
    o.raw("{\"status\":")
        .str(st)
        .raw(",\"text\":")
        .str(&format!("isotopes {st}: {}", proof_text(&[p])))
        .raw("}");
}

/// `prediction` of `detail`: the level of the rule that predicted the formula, how it was reached and the other ways to it.
pub(super) fn prediction(o: &mut Out, c: &Cand) {
    let Some(h) = c.hyp.as_ref().filter(|h| (1..=4).contains(&h.level)) else {
        o.raw("null");
        return;
    };
    let level = c.pred().unwrap_or(h.level);
    let (ok, why) = c.confirmation();
    let t = format!(
        "{} ({}): {}",
        h.label,
        h.sources.join(", "),
        PRED_TEXT[usize::from(level)]
    );
    o.raw("{\"level\":")
        .int(i64::from(level))
        .raw(",\"text\":")
        .str(&t)
        .raw(",\"evidence\":");
    strs(
        o,
        &h.alternatives
            .iter()
            .map(|a| format!("also reached as {a}"))
            .chain(std::iter::once(format!(
                "{}{why}",
                if ok {
                    "data: "
                } else {
                    "data not confirming: "
                }
            )))
            .collect::<Vec<_>>(),
    );
    o.raw("}");
}
