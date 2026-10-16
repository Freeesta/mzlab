//! Shared builders of the proofs of `gruppi`: files of `campioni/2`, cells and the synthetic scenario of the fixture. Synthetic data only.
#![allow(dead_code)]

use mzlab_core::campioni::{self, number_replicates, Campione};
use mzlab_core::json::Out;
use mzlab_feat::gruppi::{GroupOpts, GroupTable, RowMeta};
use mzlab_feat::table::{CELL_ABSENT, CELL_DETECTED, CELL_FILLED};

/// A file of `kind` in `series` (`""` = shared) at `minutes` (None = unknown), injected at position `order`.
pub fn file(name: &str, kind: &str, series: &str, minutes: Option<f64>, order: u32) -> Campione {
    let mut c = Campione::new(name, "", order);
    c.kind = kind.into();
    c.series = series.into();
    c.set_time(minutes, "min");
    c
}

/// Files with the replicates numbered.
pub fn files(mut v: Vec<Campione>) -> Vec<Campione> {
    number_replicates(&mut v);
    v
}

/// A cell: detected with area `a`, filled with area `a`, absent.
pub fn d(a: f64) -> (u8, f64) {
    (CELL_DETECTED, a)
}
pub fn f(a: f64) -> (u8, f64) {
    (CELL_FILLED, a)
}
pub fn x() -> (u8, f64) {
    (CELL_ABSENT, 0.0)
}

pub fn meta(id: u32, name: &str, mz: f64) -> RowMeta {
    RowMeta {
        id,
        tp_name: name.into(),
        mz,
        polarity: "+".into(),
        rt: 5.0,
        ..RowMeta::default()
    }
}

/// A table from rows of cells (one cell per file).
pub fn table(rows: Vec<(RowMeta, Vec<(u8, f64)>)>, n_files: usize) -> GroupTable {
    let mut t = GroupTable {
        n_rows: rows.len(),
        n_files,
        ..GroupTable::default()
    };
    for (m, cells) in rows {
        assert_eq!(cells.len(), n_files, "{}", m.tp_name);
        for (s, a) in cells {
            t.state.push(s);
            t.area.push(a);
            t.height.push(a / 10.0);
            t.apex_rt.push(5.0);
        }
        t.rows.push(m);
    }
    t
}

/// The table with its columns in another order; `perm[new] = old`. The files must be permuted by the caller in the same way.
pub fn permute(tb: &GroupTable, perm: &[usize]) -> GroupTable {
    let mut t = tb.clone();
    for r in 0..tb.n_rows {
        for (new, &old) in perm.iter().enumerate() {
            t.area[r * tb.n_files + new] = tb.area[r * tb.n_files + old];
            t.height[r * tb.n_files + new] = tb.height[r * tb.n_files + old];
            t.state[r * tb.n_files + new] = tb.state[r * tb.n_files + old];
            t.apex_rt[r * tb.n_files + new] = tb.apex_rt[r * tb.n_files + old];
        }
    }
    t
}

/// The synthetic degradation of the fixture: a shared blank, time zero and parent standard, then six conditions at the end point only
/// (`Air` in triplicate). Files 0..=10.
pub fn scenario_files() -> Vec<Campione> {
    files(vec![
        file("blank.mzML", "blank", "", None, 1),
        file("t0.mzML", "t0", "", Some(0.0), 2),
        file("std.mzML", "std", "", None, 3),
        file("O2_60.mzML", "sample", "O2", Some(60.0), 4),
        file("Air_60_a.mzML", "sample", "Air", Some(60.0), 5),
        file("Air_60_b.mzML", "sample", "Air", Some(60.0), 6),
        file("Air_60_c.mzML", "sample", "Air", Some(60.0), 7),
        file("N2_60.mzML", "sample", "N2", Some(60.0), 8),
        file("Ar_60.mzML", "sample", "Ar", Some(60.0), 9),
        file("CO2_60.mzML", "sample", "CO2", Some(60.0), 10),
        file("NoGas_60.mzML", "sample", "NoGas", Some(60.0), 11),
    ])
}

/// Cells in the file order: blank, t0, std, O2, Air a/b/c, N2, Ar, CO2, NoGas.
type Cells = Vec<(u8, f64)>;

/// Cells from text: `-` absent, `~5e3` filled with that area, anything else detected with that area.
fn cells(text: &str) -> Cells {
    text.split_whitespace()
        .map(|t| match t.strip_prefix('~') {
            _ if t == "-" => x(),
            Some(a) => f(a.parse().unwrap()),
            None => d(t.parse().unwrap()),
        })
        .collect()
}

pub fn scenario_rows() -> Vec<(RowMeta, Cells)> {
    let mut parent = meta(0, "Parent", 364.4);
    parent.is_parent = true;
    let mut ms2 = meta(11, "TP 355", 355.2);
    ms2.ms2_files = vec![3];
    let rows = [
        (parent, "- 1e7 1e7 2e6 5e6 5.2e6 4.8e6 6e6 8e6 7e6 9e6"),
        (
            meta(1, "TP 362", 362.2),
            "- - - 1.2e6 6e5 6.6e5 5.4e5 8e5 4e5 9e5 3e5",
        ),
        (meta(2, "TP 305", 305.3), "- - - 5e5 ~3e3 - - - ~1e3 - -"),
        (
            meta(3, "TP 229", 229.1),
            "- 4e5 - 1e6 9e5 9e5 9e5 7e5 - - -",
        ),
        (meta(4, "TP 224", 224.2), "- - 3e5 1e6 - - - 8e5 - - -"),
        (meta(5, "TP 391", 391.3), "- - - - 2e5 2.2e5 - 3e5 - - -"),
        (meta(6, "TP 280", 280.2), "- - - 4e5 1e5 - - 3e5 - - -"),
        (meta(7, "TP 301", 301.2), "- - - 7e5 5e5 5e5 5e5 6e5 - - -"),
        (meta(8, "TP 410", 410.4), "- - - ~2e3 ~1e3 ~1e3 - - - - -"),
        (meta(9, "TP 150", 150.1), "5e5 - - 1e6 - - - 8e5 - - -"),
        (meta(10, "TP 333", 333.3), "- - - ~7e4 3e5 3e5 3e5 - - - -"),
        (ms2, "- - - 6e5 - - - - 5e5 - -"),
    ];
    rows.into_iter().map(|(m, c)| (m, cells(c))).collect()
}

pub fn scenario_opts() -> GroupOpts {
    GroupOpts {
        reference: Some("O2".into()),
        factors: vec![("O2".into(), vec![("catalyst".into(), "TiO2 P25".into())])],
        ..GroupOpts::default()
    }
}

pub fn scenario() -> (GroupTable, Vec<Campione>, GroupOpts) {
    (
        table(scenario_rows(), 11),
        scenario_files(),
        scenario_opts(),
    )
}

fn matrix(o: &mut Out, tb: &GroupTable, cell: impl Fn(usize) -> f64) {
    o.raw("[");
    for r in 0..tb.n_rows {
        o.raw(if r > 0 { ",[" } else { "[" });
        for c in 0..tb.n_files {
            o.raw(if c > 0 { "," } else { "" })
                .num(cell(r * tb.n_files + c));
        }
        o.raw("]");
    }
    o.raw("]");
}

/// The JSON input of `run_json` for a table, its files and options.
pub fn input_json(tb: &GroupTable, cs: &[Campione], opts: &GroupOpts) -> String {
    let mut o = Out::default();
    o.raw("{\"table\":{\"n_rows\":").int(tb.n_rows as i64);
    o.raw(",\"n_files\":")
        .int(tb.n_files as i64)
        .raw(",\"rows\":[");
    for (i, m) in tb.rows.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" })
            .raw("{\"id\":")
            .int(m.id as i64);
        o.raw(",\"tp_name\":")
            .str(&m.tp_name)
            .raw(",\"mz\":")
            .num(m.mz);
        o.raw(",\"polarity\":")
            .str(&m.polarity)
            .raw(",\"rt\":")
            .num(m.rt);
        o.raw(",\"is_parent\":")
            .raw(if m.is_parent { "true" } else { "false" });
        o.raw(",\"ms2_files\":[");
        let ms2: Vec<String> = m.ms2_files.iter().map(|f| f.to_string()).collect();
        o.raw(&ms2.join(",")).raw("]}");
    }
    o.raw("],\"area\":");
    matrix(&mut o, tb, |i| tb.area[i]);
    o.raw(",\"height\":");
    matrix(&mut o, tb, |i| tb.height[i]);
    o.raw(",\"state\":");
    matrix(&mut o, tb, |i| f64::from(tb.state[i]));
    o.raw(",\"apex_rt\":");
    matrix(&mut o, tb, |i| f64::from(tb.apex_rt[i]));
    o.raw("},\"campioni\":").raw(&campioni::to_json(cs));
    o.raw(",\"opts\":{\"reference\":");
    match &opts.reference {
        Some(r) => o.str(r),
        None => o.raw("null"),
    };
    o.raw(",\"min_rep_frac\":").num(opts.min_rep_frac);
    o.raw(",\"min_area\":").num(opts.min_area);
    o.raw(",\"ctrl_ratio\":").num(opts.ctrl_ratio);
    o.raw(",\"norm\":").str(&opts.norm);
    o.raw(",\"shape_drop\":")
        .num(opts.shape_drop)
        .raw(",\"factors\":{");
    for (i, (c, kv)) in opts.factors.iter().enumerate() {
        o.raw(if i > 0 { "," } else { "" }).str(c).raw(":{");
        for (j, (k, v)) in kv.iter().enumerate() {
            o.raw(if j > 0 { "," } else { "" }).str(k).raw(":").str(v);
        }
        o.raw("}");
    }
    o.raw("}}}");
    o.0
}
