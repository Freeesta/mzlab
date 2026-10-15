#![cfg(feature = "attributio")]
//! Parity of the review proofs with Attributio (sub-formula explanation, isotope pattern, MS2 purity, raw proofs, RT versus logP: 1e-9 relative) and the
//! agreement of the fragmenter with the reference (a measure, with a floor).

mod attributio_comune;
use attributio_comune::golden;
use mzlab_feat::attributio::review::explain::{
    explain, fmt_formula, iso_pattern, parse_formula, prep_ms2, removed_reasons, Formula,
};
use mzlab_feat::attributio::review::fragmenter::fragment_molecule;
use mzlab_feat::attributio::review::isotopes::{clusters, compare, pattern, IsoTol};
use mzlab_feat::attributio::review::matching::Ms2;
use mzlab_feat::attributio::review::purity::*;
use mzlab_feat::attributio::review::raw::{coelution, ms1_metrics, peak_quality, ScanInfo};
use mzlab_feat::attributio::review::rtlogp::*;
use mzlab_feat::attributio::review::settings::ReviewSettings;
use serde_json::Value;

fn f(v: &Value) -> f64 {
    v.as_f64().unwrap_or(f64::NAN)
}
fn fs(v: &Value) -> Vec<f64> {
    v.as_array().expect("array").iter().map(f).collect()
}
fn close(got: f64, want: &Value, at: &str) {
    match want.as_f64() {
        None => assert!(got.is_nan(), "{at}: {got} where null"),
        Some(w) => assert!(
            (got - w).abs() <= 1e-9 * got.abs().max(w.abs()).max(1e-6) || got == w,
            "{at}: {got} != {w}"
        ),
    }
}
fn opt(got: Option<f64>, want: &Value, at: &str) {
    match (got, want.as_f64()) {
        (None, None) => {}
        (Some(g), Some(_)) => close(g, want, at),
        (g, w) => panic!("{at}: {g:?} != {w:?}"),
    }
}
fn formula(v: &Value) -> Formula {
    v.as_object()
        .expect("formula")
        .iter()
        .map(|(k, n)| (k.clone(), n.as_i64().expect("count")))
        .collect()
}
fn ms2(mz: &Value, it: &Value, prec: f64) -> Ms2 {
    Ms2 {
        mz: fs(mz),
        it: fs(it),
        prec,
    }
}

#[test]
fn explanations_equal_the_reference_tool() {
    let g = golden("sintetica", "spiegazioni.json");
    let s = ReviewSettings::default();
    for (k, c) in g["spiegazioni"]
        .as_array()
        .expect("cases")
        .iter()
        .enumerate()
    {
        let at = format!("case {k} {}", c["formula"]);
        let (mz, pol) = (f(&c["mz"]), c["pol"].as_str().expect("pol"));
        let (rm, ri) = (fs(&c["raw_mz"]), fs(&c["raw_it"]));
        let (em, ei, _) = prep_ms2(&rm, &ri, mz, s.rel_min);
        assert_eq!(em, fs(&c["prep_mz"]), "{at}: cleaned m/z");
        assert_eq!(ei, fs(&c["prep_it"]), "{at}: cleaned intensities");
        let rem: Vec<(usize, &str)> = removed_reasons(&rm, &ri, mz, s.rel_min)
            .iter()
            .map(|(i, w)| (*i, w.label()))
            .collect();
        let want: Vec<(usize, String)> = c["removed"]
            .as_array()
            .expect("removed")
            .iter()
            .map(|r| {
                (
                    r[0].as_u64().expect("i") as usize,
                    r[1].as_str().expect("w").to_string(),
                )
            })
            .collect();
        assert_eq!(
            rem.iter()
                .map(|r| (r.0, r.1.to_string()))
                .collect::<Vec<_>>(),
            want,
            "{at}: removed"
        );
        let ex = explain(
            Some(&parse_formula(c["formula"].as_str().expect("f"))),
            mz,
            pol,
            (&em, &ei),
            &s,
        );
        assert_eq!(ex.adduct.as_deref(), c["adduct"].as_str(), "{at}: adduct");
        assert_eq!(
            ex.ion_formula.as_deref(),
            c["ion_formula"].as_str(),
            "{at}: ion formula"
        );
        close(ex.ion_ppm, &c["ion_ppm"], &format!("{at}: ion ppm"));
        assert_eq!(
            ex.n_expl as i64,
            c["n_expl"].as_i64().expect("n"),
            "{at}: n explained"
        );
        close(ex.frac_expl, &c["frac_expl"], &format!("{at}: fraction"));
        close(ex.decoy_frac, &c["decoy_frac"], &format!("{at}: decoy"));
        if let (Some(ion), Some(w)) = (&ex.ion, c["iso"].as_array()) {
            let (a, b) = iso_pattern(ion);
            close(a, &w[0], &format!("{at}: M+1"));
            close(b, &w[1], &format!("{at}: M+2"));
        }
        for (i, h) in c["frags"].as_array().expect("frags").iter().enumerate() {
            match (&ex.frags[i], h.is_null()) {
                (None, true) => {}
                (Some(x), false) => {
                    assert_eq!(
                        x.formula,
                        h["formula"].as_str().expect("formula"),
                        "{at}: fragment {i} formula"
                    );
                    assert_eq!(
                        x.loss,
                        h["loss"].as_str().expect("loss"),
                        "{at}: fragment {i} loss"
                    );
                    close(x.mz_th, &h["mz_th"], &format!("{at}: fragment {i} m/z"));
                    // `assign`: same formula, ppm within 0.005
                    assert!(
                        (x.ppm - f(&h["ppm"])).abs() <= 0.005,
                        "{at}: fragment {i} ppm {} vs {}",
                        x.ppm,
                        h["ppm"]
                    );
                }
                (a, _) => panic!("{at}: fragment {i}: {a:?} vs {h}"),
            }
        }
    }
}

#[test]
fn isotope_patterns_equal_the_reference_tool() {
    let g = golden("sintetica", "spiegazioni.json");
    for (k, c) in g["isotopi"].as_array().expect("cases").iter().enumerate() {
        let (ion, pol) = (formula(&c["ion"]), c["pol"].as_str().expect("pol"));
        let at = format!("ion {k} {}", fmt_formula(&ion));
        let sticks = pattern(&ion, pol, 1e-3);
        let want = c["sticks"].as_array().expect("sticks");
        assert_eq!(sticks.len(), want.len(), "{at}: sticks");
        for (i, (m, a)) in sticks.iter().enumerate() {
            close(*m, &want[i][0], &format!("{at}: stick {i} m/z"));
            close(*a, &want[i][1], &format!("{at}: stick {i} abundance"));
        }
        for (cl, w) in clusters(&sticks, 4)
            .iter()
            .zip(c["clusters"].as_array().expect("clusters"))
        {
            for (name, got) in [
                ("mz", cl.mz),
                ("lo", cl.lo),
                ("hi", cl.hi),
                ("rel", cl.rel),
                ("main", cl.main),
            ] {
                close(got, &w[name], &format!("{at}: cluster {} {name}", cl.k));
            }
        }
        let cmp = compare(
            &ion,
            pol,
            &fs(&c["obs_mz"]),
            &fs(&c["obs_it"]),
            f(&c["prec"]),
            IsoTol {
                abs: 0.06,
                rel: 0.35,
                ppm: 10.0,
            },
            Some(12.0),
        );
        let w = &c["cmp"];
        let Some(cmp) = cmp else {
            assert!(w.is_null(), "{at}: comparison");
            continue;
        };
        assert_eq!(
            cmp.verdict,
            w["verdict"].as_str().expect("verdict"),
            "{at}: verdict"
        );
        opt(cmp.rmse, &w["rmse"], &format!("{at}: rmse"));
        opt(cmp.a_ppm, &w["a_ppm"], &format!("{at}: A ppm"));
        assert_eq!(
            cmp.flags,
            w["flags"]
                .as_array()
                .expect("flags")
                .iter()
                .map(|x| x.as_str().expect("flag").to_string())
                .collect::<Vec<_>>(),
            "{at}: flags"
        );
        assert_eq!(
            cmp.rows.len(),
            w["rows"].as_array().expect("rows").len(),
            "{at}: rows"
        );
        for (r, wr) in cmp.rows.iter().zip(w["rows"].as_array().expect("rows")) {
            assert_eq!(
                r.verdict,
                wr["verdict"].as_str().expect("v"),
                "{at}: row {} verdict",
                r.k
            );
            close(r.obs, &wr["obs"], &format!("{at}: row {} obs", r.k));
            close(r.th, &wr["th"], &format!("{at}: row {} th", r.k));
            opt(r.diff, &wr["diff"], &format!("{at}: row {} diff", r.k));
            opt(r.obs_mz, &wr["obs_mz"], &format!("{at}: row {} m/z", r.k));
            opt(r.ppm, &wr["ppm"], &format!("{at}: row {} ppm", r.k));
        }
    }
}

#[test]
fn purity_and_raw_proofs_equal_the_reference_tool() {
    let g = golden("sintetica", "spiegazioni.json");
    let p = &g["purezza"];
    let i = &p["inputs"];
    let prec = f(&i["prec"]);
    let sticks = [(prec, 100.0), (prec + 1.00336, 17.0)];
    let iso = isolation_purity(
        &fs(&i["mz"]),
        &fs(&i["it"]),
        prec,
        prec,
        1.0,
        1.0,
        Some(&sticks),
        "POS",
        10.0,
        0.10,
    )
    .expect("isolation");
    opt(iso.purity, &p["iso"]["purity"], "purity");
    close(iso.prec_int, &p["iso"]["prec_int"], "precursor intensity");
    assert_eq!(
        iso.co.len(),
        p["iso"]["co"].as_array().expect("co").len(),
        "co-isolated ions"
    );
    for (c, w) in iso.co.iter().zip(p["iso"]["co"].as_array().expect("co")) {
        close(c.mz, &w["mz"], "co mz");
        close(c.rel, &w["rel"], "co rel");
        assert_eq!(c.kind, w["kind"].as_str().expect("kind"));
    }
    let iso2 = isolation_purity(
        &fs(&i["mz"]),
        &fs(&i["it"]),
        prec,
        prec,
        1.5,
        1.5,
        None,
        "POS",
        10.0,
        0.10,
    );
    assert_eq!(
        iso2.as_ref().map(|x| x.co.len()),
        Some(p["iso2"]["co"].as_array().expect("co").len())
    );
    let imp = impossible_losses(&fs(&i["pm"]), &fs(&i["pi"]), prec, 0.05, 0.005, None);
    let wi = p["imposs"].as_array().expect("imposs");
    assert_eq!(imp.len(), wi.len(), "impossible losses");
    for (a, w) in imp.iter().zip(wi) {
        close(a.mz, &w["mz"], "imposs mz");
        close(a.rel, &w["rel"], "imposs rel");
        close(a.loss, &w["loss"], "imposs loss");
        assert_eq!(a.why, w["why"].as_str().expect("why"));
    }
    let expl: Vec<bool> = i["expl"]
        .as_array()
        .expect("expl")
        .iter()
        .map(|b| b.as_bool().expect("bool"))
        .collect();
    let (em, ei) = (ms2(&i["em"], &i["ei"], prec), 0);
    let _ = ei;
    let rf = ms2(&i["rm"], &i["ri"], prec);
    let mut scores = Vec::new();
    for (k, (low, shift)) in [(None, 0.0), (Some(70.0), 0.0), (Some(70.0), 14.01565)]
        .into_iter()
        .enumerate()
    {
        let q = candidate_scores(&em, &rf, 0.02, low, shift, &expl);
        let w = &p["qs"][k];
        for (n, got) in [
            ("fwd", q.fwd),
            ("rev", q.rev),
            ("presence", q.presence),
            ("cos_expl", q.cos_expl),
            ("p_chance", q.p_chance),
        ] {
            close(got, &w[n], &format!("scores {k} {n}"));
        }
        for (n, got) in [
            ("n_main", q.n_main),
            ("matched_expl", q.matched_expl),
            ("informative", q.informative),
            ("matched", q.matched),
        ] {
            assert_eq!(got, w[n].as_i64().expect("count"), "scores {k} {n}");
        }
        let li = low_information(Some(&q), 3).expect("low information");
        assert_eq!(
            (li.low, li.why.as_str()),
            (
                p["low"][k]["low"].as_bool().expect("low"),
                p["low"][k]["why"].as_str().expect("why")
            )
        );
        scores.push(q);
    }
    close(
        unexplained(&em.it, &expl, None),
        &p["unexplained"][0],
        "unexplained",
    );
    close(
        unexplained(&em.it, &expl, Some(&[false, true, false, false, false])),
        &p["unexplained"][1],
        "unexplained, matched",
    );
    for (k, (n, r, m)) in [(5, 6, 3), (30, 40, 12), (8, 8, 8), (1, 1, 0)]
        .into_iter()
        .enumerate()
    {
        close(
            chance_probability(n, r, m, 0.02, 400.0),
            &p["chance"][k],
            &format!("chance {k}"),
        );
    }
    let coel: Vec<CoelRow> = p["coel_rows"]
        .as_array()
        .expect("rows")
        .iter()
        .map(|w| CoelRow {
            mz: f(&w["mz"]),
            rel: f(&w["rel"]),
            r: w["r"].as_f64(),
            offset: w["offset"].as_f64(),
        })
        .collect();
    let bad = badge(Some(&iso), &coel, Some(0.55), &imp, Some(&scores[0]));
    assert_eq!(
        (bad.state, bad.reasons),
        (
            p["badge_bad"]["state"].as_str().expect("state"),
            strs(&p["badge_bad"]["reasons"])
        )
    );
    let good = badge(None, &[], Some(0.1), &[], Some(&scores[1]));
    assert_eq!(
        (good.state, good.reasons),
        (
            p["badge_good"]["state"].as_str().expect("state"),
            strs(&p["badge_good"]["reasons"])
        )
    );
    raw_proofs(p, i);
}

fn strs(v: &Value) -> Vec<String> {
    v.as_array()
        .expect("strings")
        .iter()
        .map(|x| x.as_str().expect("text").to_string())
        .collect()
}

fn raw_proofs(p: &Value, i: &Value) {
    let (rt, g1, g2, y, blank) = (
        fs(&i["rt"]),
        fs(&i["g1"]),
        fs(&i["g2"]),
        fs(&i["y"]),
        fs(&i["blank"]),
    );
    let zeros = vec![0.0; rt.len()];
    let cases = [
        coelution(&rt, &g1, &g2),
        coelution(&rt, &g1, &zeros),
        coelution(&rt[..3], &g1[..3], &g2[..3]),
    ];
    for (k, c) in cases.iter().enumerate() {
        let w = &p["coelution"][k];
        match c {
            Some(c) => {
                close(c.offset, &w["offset"], &format!("coelution {k} offset"));
                close(c.r, &w["r"], &format!("coelution {k} r"));
            }
            None => assert!(w.is_null(), "coelution {k}"),
        }
    }
    let q = [
        peak_quality(&rt, &y, 4.3, Some((&rt, &blank)), 0.15),
        peak_quality(&rt, &y, 4.3, None, 0.15),
        peak_quality(&rt, &zeros, 4.3, None, 0.15),
    ];
    for (k, c) in q.iter().enumerate() {
        let w = &p["peak_quality"][k];
        let Some(c) = c else {
            assert!(w.is_null(), "peak quality {k}");
            continue;
        };
        for (n, got) in [("apex_rt", c.apex_rt), ("apex", c.apex), ("fwhm", c.fwhm)] {
            close(got, &w[n], &format!("peak quality {k} {n}"));
        }
        opt(c.asym, &w["asym"], "asymmetry");
        opt(c.s2b, &w["s2b"], "signal to blank");
        opt(c.blank_max, &w["blank_max"], "blank maximum");
        assert_eq!(
            (c.points, c.points_half),
            (
                w["points"].as_i64().expect("n"),
                w["points_half"].as_i64().expect("n")
            )
        );
    }
    let info = ScanInfo {
        low: f(&p["ms1_info"]["low"]),
        high: f(&p["ms1_info"]["high"]),
        target: f(&p["ms1_info"]["target"]),
        lo: f(&p["ms1_info"]["lo"]),
        hi: f(&p["ms1_info"]["hi"]),
        ms1_mz: fs(&p["ms1_info"]["ms1_mz"]),
        ms1_it: fs(&p["ms1_info"]["ms1_it"]),
    };
    let ion = formula(&p["ms1_ion"]);
    let s = ReviewSettings::default();
    let prec = f(&i["prec"]);
    let ms1 = [
        ms1_metrics(Some(&info), prec, Some(&ion), &s),
        ms1_metrics(Some(&info), 400.0, Some(&ion), &s),
        ms1_metrics(Some(&info), prec, None, &s),
    ];
    for (k, m) in ms1.iter().enumerate() {
        let w = &p["ms1"][k];
        let Some(m) = m else {
            assert!(
                w.is_null() || w.as_object().is_none_or(|o| o.is_empty()),
                "ms1 {k}"
            );
            continue;
        };
        assert_eq!(m.iso_ok, w["iso_ok"].as_bool(), "ms1 {k} isotope verdict");
        assert_eq!(
            m.chim_n as i64,
            w["chim_n"].as_i64().expect("n"),
            "ms1 {k} chimeric ions"
        );
        for (n, got) in [("obs1", m.obs1), ("obs2", m.obs2), ("chim_max", m.chim_max)] {
            close(got, &w[n], &format!("ms1 {k} {n}"));
        }
        if w["th1"].is_null() {
            assert!(m.th1.is_nan());
        } else {
            close(m.th1, &w["th1"], &format!("ms1 {k} th1"));
            close(m.th2, &w["th2"], &format!("ms1 {k} th2"));
        }
    }
}

fn fit_matches(got: &Fit, w: &Value, at: &str) {
    close(got.a, &w["a"], &format!("{at}: a"));
    close(got.mad, &w["mad"], &format!("{at}: mad"));
    close(got.r2, &w["r2"], &format!("{at}: r2"));
    assert_eq!(got.n as i64, w["n"].as_i64().expect("n"), "{at}: n");
    match w["b"].as_array() {
        Some(bs) => got
            .b
            .iter()
            .zip(bs)
            .enumerate()
            .for_each(|(k, (g, w))| close(*g, w, &format!("{at}: b{k}"))),
        None => close(got.b[0], &w["b"], &format!("{at}: b")),
    }
}

#[test]
fn rt_versus_logp_equals_the_reference_tool() {
    let g = golden("sintetica", "spiegazioni.json");
    let r = &g["rtlogp"];
    let pts = r["points"].as_array().expect("points");
    let x: Vec<f64> = pts.iter().map(|p| f(&p["xlogp3"])).collect();
    let y: Vec<f64> = pts.iter().map(|p| f(&p["y"])).collect();
    // the line and the regression were fitted on the raw lists of the reference: rebuild them from the points
    fit_matches(&fit_line(&x, &y).expect("line"), &r["line"], "line");
    assert!(fit_line(&x[..2], &y[..2]).is_none() && r["line_short"].is_null());
    let design: Vec<Vec<f64>> = pts
        .iter()
        .map(|p| {
            ["xlogp3", "mw", "tpsa", "nrot", "hbd", "hba"]
                .iter()
                .map(|k| f(&p[*k]))
                .collect()
        })
        .collect();
    fit_matches(&fit_mlr(&design, &y).expect("mlr"), &r["mlr"], "mlr");
    assert!(fit_mlr(&design[..5], &y[..5]).is_none() && r["mlr_short"].is_null());
    let cal: Vec<CalPoint> = pts
        .iter()
        .map(|p| CalPoint {
            logp: LogPValues {
                xlogp3: p["xlogp3"].as_f64(),
                crippen: p["crippen"].as_f64(),
                class: p["class"].as_str().map(String::from),
                mw: p["mw"].as_f64(),
                tpsa: p["tpsa"].as_f64(),
                hbd: p["hbd"].as_f64(),
                hba: p["hba"].as_f64(),
                nrot: p["nrot"].as_f64(),
            },
            y: p["y"].as_f64(),
        })
        .collect();
    let model = calibrate(&cal, 3.0, 0.0, MIN_N);
    let small = calibrate(&cal[..10], 3.0, 0.0, MIN_N);
    for (m, w, at) in [
        (&model, &r["model"], "model"),
        (&small, &r["model_small"], "small model"),
    ] {
        assert_eq!(
            (m.source, m.n as i64, m.note.as_str()),
            (
                w["source"].as_str().expect("s"),
                w["n"].as_i64().expect("n"),
                w["note"].as_str().expect("note")
            ),
            "{at}"
        );
        opt(m.window, &w["window"], at);
        match &m.fit {
            Some(fit) => fit_matches(fit, &w["fit"], at),
            None => assert!(w["fit"].is_null(), "{at}"),
        }
        assert_eq!(
            m.per_class.len(),
            w["per_class"].as_object().expect("classes").len(),
            "{at}: classes"
        );
    }
    let lp = LogPValues {
        xlogp3: Some(2.5),
        crippen: Some(2.4),
        class: Some("base".into()),
        mw: Some(220.0),
        tpsa: Some(30.0),
        hbd: Some(1.0),
        hba: Some(4.0),
        nrot: Some(3.0),
    };
    let none = LogPValues {
        xlogp3: None,
        crippen: None,
        ..LogPValues::default()
    };
    let checks = [
        check(&model, &lp, Some(3.6), 3.0, 0.0),
        check(&model, &lp, Some(9.0), 3.0, 0.0),
        check(&model, &none, Some(3.0), 3.0, 0.0),
        check(&small, &lp, Some(3.0), 3.0, 0.0),
    ];
    for (k, c) in checks.iter().enumerate() {
        let w = &r["checks"][k];
        opt(c.pred, &w["pred"], &format!("check {k} prediction"));
        opt(c.resid, &w["resid"], &format!("check {k} residual"));
        assert_eq!(c.ok, w["ok"].as_bool(), "check {k}");
    }
    let grad: Vec<Vec<f64>> = r["gradient"]
        .as_array()
        .expect("gradient")
        .iter()
        .map(fs)
        .collect();
    for w in r["organic"].as_array().expect("organic") {
        close(
            organic_fraction(f(&w[0]), &grad, 0.4),
            &w[1],
            "organic fraction",
        );
    }
}

#[test]
fn the_fragmenter_agrees_with_the_reference_tool() {
    let g = golden("sintetica", "frammenti.json");
    let (mut total, mut equal, mut bonds_equal, mut bonds_total) = (0usize, 0usize, 0usize, 0usize);
    let mut below: Vec<String> = Vec::new();
    for m in g["molecole"].as_array().expect("molecules") {
        if !m["ok"].as_bool().unwrap_or(false) {
            continue;
        }
        let smi = m["smiles"].as_str().expect("smiles");
        let r = fragment_molecule(
            smi,
            m["pol"].as_str().expect("pol"),
            &fs(&m["peaks"]),
            10.0,
            25,
        )
        .unwrap_or_default();
        let want = m["explained"].as_array().expect("explained");
        let eq = want
            .iter()
            .filter(|w| {
                r.explained.iter().any(|e| {
                    e.peak == f(&w[0])
                        && e.formula == w[1].as_str().expect("f")
                        && e.loss == w[4].as_str().expect("loss")
                        && (e.mz_th - f(&w[2])).abs() < 1e-5
                })
            })
            .count();
        total += want.len();
        equal += eq;
        let wb = m["bonds"].as_array().expect("bonds");
        bonds_total += wb.len();
        bonds_equal += wb
            .iter()
            .filter(|b| {
                r.bonds
                    .iter()
                    .any(|x| x.label == b[0].as_str().expect("label"))
            })
            .count();
        if eq < want.len() {
            below.push(format!("{smi}: {eq}/{}", want.len()));
        }
    }
    let frac = equal as f64 / total as f64;
    println!("fragmenter: {equal}/{total} equal explained fragments = {frac:.4}; bonds cut {bonds_equal}/{bonds_total}; differences: {below:?}");
    assert!(
        frac >= 0.90,
        "fraction of equal explained fragments {frac:.3}: {below:?}"
    );
}
