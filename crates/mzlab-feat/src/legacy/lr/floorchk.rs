//! Rule R1 of docs/studies/TP_SCREENING_RULES.md in the Dig: the one-ion noise floor (`mzlab_core::ionstat`) of the XIC of every checked cell
//! (`peakchk::floor_of`) as `points[].sn_floor`, a `noise_floor` entry of `rows[].checks` on the cell the peak criterion names, and the
//! `noise_floor` block of the summary with the automatic choice and one log line per row that warns or fails. Evidence only (owner decision D1
//! open): no state, criterion, score or label reads it, so every existing result stays as it was.

use mzlab_core::ionstat::{FloorResult, OneIon};
use mzlab_core::json::{round, Out};

use super::{Entry, LrDig};

pub const ID: &str = "noise_floor";
const STATUSES: [&str; 4] = ["pass", "warn", "fail", "n/a"];

/// The `noise_floor` check of one row.
pub struct RowCheck<'a> {
    pub status: &'static str,
    /// The column of the cell the check reads and its floor.
    pub cell: Option<(usize, &'a FloorResult)>,
    pub rule: String,
    /// The status of every checked cell of the row, in the order of `STATUSES`.
    pub cells: [usize; 4],
}

/// The S/N of the peak of cell (r, k) on the one-ion floor.
pub fn sn_floor(d: &LrDig, r: usize, k: usize) -> Option<f64> {
    let f = d.checks.floor.get(d.table.cell(r, k))?.as_ref()?;
    f.peak.as_ref()?.sn
}

fn slot(status: &str) -> usize {
    STATUSES.iter().position(|s| *s == status).unwrap_or(3)
}

/// The check of row `e`: on the cell `peakchk::criterion` shows (the best peak by S/N, or the best rejected one when none passed).
pub fn row<'a>(d: &'a LrDig, e: &Entry) -> RowCheck<'a> {
    let mut cells = [0; 4];
    let Some(r) = e.row.filter(|&r| r < d.table.len()) else {
        return RowCheck {
            status: "n/a",
            cell: None,
            rule: "no row in the table: no XIC to measure".into(),
            cells,
        };
    };
    let c = &d.checks;
    let qs: Vec<(usize, f64, bool)> = (0..d.table.n_files)
        .filter_map(|k| c.of(&d.table, r, k).map(|q| (k, q.snr, q.ok())))
        .collect();
    for &(k, _, _) in &qs {
        let f = c.floor[d.table.cell(r, k)].as_ref();
        cells[slot(f.and_then(|f| f.check.as_ref()).map_or("n/a", |x| x.status))] += 1;
    }
    let good = qs.iter().any(|x| x.2);
    let best = qs
        .iter()
        .filter(|x| x.2 || !good)
        .max_by(|a, b| a.1.total_cmp(&b.1));
    let Some(&(k, _, _)) = best else {
        return RowCheck {
            status: "n/a",
            cell: None,
            rule: "no cell of the row had a peak check: no XIC to measure".into(),
            cells,
        };
    };
    match c.floor[d.table.cell(r, k)].as_ref() {
        Some(f) => {
            let ck = f.check.as_ref();
            RowCheck {
                status: ck.map_or("n/a", |x| x.status),
                cell: Some((k, f)),
                rule: ck.map_or(String::new(), |x| x.rule.clone()),
                cells,
            }
        }
        None => RowCheck {
            status: "n/a",
            cell: None,
            rule: "no background point outside the peak window".into(),
            cells,
        },
    }
}

fn opt_r(o: &mut Out, v: Option<f64>, nd: usize) {
    o.opt(v.filter(|x| x.is_finite()).map(|x| round(x, nd)));
}

/// `,"checks":[{id, status, value, threshold, evidence, rule}]` of row `e` (the contract of B.1 of the study).
pub fn json(o: &mut Out, d: &LrDig, e: &Entry) {
    let ck = row(d, e);
    let f = ck.cell.map(|x| x.1);
    let pk = f.and_then(|f| f.peak.as_ref());
    o.raw(",\"checks\":[{\"id\":")
        .str(ID)
        .raw(",\"status\":")
        .str(ck.status)
        .raw(",\"value\":");
    opt_r(o, pk.and_then(|p| p.sn), 2);
    o.raw(",\"threshold\":")
        .num(d.settings.min_snr)
        .raw(",\"evidence\":{\"file\":");
    match ck.cell {
        Some((k, _)) => o.str(&d.lc[k].name),
        None => o.raw("null"),
    };
    o.raw(",\"apex_rt\":");
    opt_r(o, pk.map(|p| p.apex_rt), 4);
    o.raw(",\"window\":");
    match pk {
        Some(p) => o
            .raw("[")
            .num(round(p.from, 4))
            .raw(",")
            .num(round(p.to, 4))
            .raw("]"),
        None => o.raw("null"),
    };
    o.raw(",\"height\":");
    opt_r(o, pk.map(|p| p.height), 1);
    o.raw(",\"sn_mad\":");
    opt_r(o, pk.and_then(|p| p.sn_mad), 2);
    o.raw(",\"ions\":");
    opt_r(o, pk.and_then(|p| p.ions), 1);
    o.raw(",\"noise\":");
    opt_r(o, f.map(|f| f.floor.noise), 3);
    o.raw(",\"estimator\":");
    match f {
        Some(f) => o.str(f.floor.estimator),
        None => o.raw("null"),
    };
    o.raw(",\"background_n\":");
    match f {
        Some(f) => o.int(f.floor.n as i64),
        None => o.raw("null"),
    };
    o.raw(",\"terms\":");
    match f {
        Some(f) => {
            let t = &f.floor;
            o.raw("{\"mad\":").num(round(t.mad, 3)).raw(",\"sd\":").num(round(t.sd, 3));
            o.raw(",\"sd_term\":").num(round(t.sd_term, 3)).raw(",\"one_ion\":");
            opt_r(o, t.one_ion, 4);
            o.raw(",\"one_ion_source\":").str(t.one_ion_source).raw(",\"lambda\":");
            opt_r(o, t.lambda, 4);
            o.raw(",\"ion_term\":");
            opt_r(o, t.ion_term, 3);
            o.raw("}")
        }
        None => o.raw("null"),
    };
    o.raw(",\"quantum\":");
    match f.and_then(|f| f.floor.quantum) {
        Some(q) => o
            .raw("{\"q\":")
            .num(round(q.q, 4))
            .raw(",\"support\":")
            .num(round(q.support, 3))
            .raw(",\"n_positive\":")
            .int(q.n_positive as i64)
            .raw(",\"level\":")
            .num(round(q.level, 2))
            .raw(",\"accepted\":")
            .raw(if q.accepted { "true" } else { "false" })
            .raw("}"),
        None => o.raw("null"),
    };
    o.raw(",\"cells\":{");
    for (i, s) in STATUSES.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .str(s)
            .raw(":")
            .int(ck.cells[i] as i64);
    }
    o.raw("}},\"rule\":").str(&ck.rule).raw("}]");
}

/// `,"noise_floor":{...}` of the summary: the parameters, how often the automatic quantum was accepted, the rows per status and the log (first
/// line the choice, then one line per row that warns or fails; the rows `n/a` are only counted).
pub fn summary_json<'a>(o: &mut Out, d: &LrDig, rows: impl Iterator<Item = &'a Entry>) {
    let s = &d.settings;
    let mut by = [0usize; 4];
    let mut lines = Vec::new();
    for e in rows {
        let ck = row(d, e);
        by[slot(ck.status)] += 1;
        if matches!(ck.status, "warn" | "fail") {
            let at = ck.cell.map_or(String::new(), |(k, f)| {
                let rt = f.peak.as_ref().map_or(String::new(), |p| format!(", apex {:.2} min", p.apex_rt));
                format!(" ({}{rt})", d.lc[k].name)
            });
            lines.push(format!("{} r={} check {ID} {}: {}{at}", e.name, e.id, ck.status, ck.rule));
        }
    }
    let fl: Vec<&FloorResult> = d.checks.floor.iter().flatten().collect();
    let accepted = fl.iter().filter(|f| f.floor.one_ion_source == "auto").count();
    let src = match s.one_ion {
        OneIon::Auto => format!("auto (the quantum of each XIC, accepted in {accepted} of {} cells)", fl.len()),
        OneIon::Cps(c) => format!("set, {} cps", super::labels::g(c)),
        OneIon::Off => "off".to_string(),
    };
    let head = format!(
        "noise floor R1 (evidence only: states, criteria, scores and labels unchanged): one_ion {src}, sd_weight {}, S/N >= {}; rows pass {}, warn {}, fail {}, n/a {}",
        super::labels::g(s.noise_sd_weight),
        super::labels::g(s.min_snr),
        by[0],
        by[1],
        by[2],
        by[3]
    );
    o.raw(",\"noise_floor\":{\"one_ion\":");
    match s.one_ion {
        OneIon::Auto => o.str("auto"),
        OneIon::Off => o.num(0.0),
        OneIon::Cps(c) => o.num(c),
    };
    o.raw(",\"sd_weight\":")
        .num(s.noise_sd_weight)
        .raw(",\"min_sn\":")
        .num(s.min_snr)
        .raw(",\"evidence_only\":true,\"cells\":")
        .int(fl.len() as i64)
        .raw(",\"quantum_accepted\":")
        .int(accepted as i64)
        .raw(",\"rows\":{");
    for (i, st) in STATUSES.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .str(st)
            .raw(":")
            .int(by[i] as i64);
    }
    o.raw("},\"log\":[").str(&head);
    for l in &lines {
        o.raw(",").str(l);
    }
    o.raw("]}");
}
