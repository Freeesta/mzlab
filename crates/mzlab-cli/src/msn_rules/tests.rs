//! `mzlab-cli msn-rules`: a synthetic MassBank record through the command, the strata, the decoys and the sums. Synthetic spectra only.

use super::somma::somma;
use crate::execute;
use mzlab_core::json::parse;

fn run(a: &[&str]) -> crate::Outcome {
    execute(&a.iter().map(|s| s.to_string()).collect::<Vec<_>>())
}

/// A MassBank record of a synthetic acetanilide-like standard: the precursor and the loss of ketene.
fn registro(id: &str, strumento: &str, modo: &str, addotto: &str, smiles: &str) -> String {
    format!(
        "ACCESSION: {id}\nRECORD_TITLE: synthetic\nLICENSE: CC0\nCH$FORMULA: C8H9NO\nCH$SMILES: {smiles}\nAC$INSTRUMENT_TYPE: {strumento}\n\
         AC$MASS_SPECTROMETRY: MS_TYPE MS2\nAC$MASS_SPECTROMETRY: ION_MODE POSITIVE\nAC$MASS_SPECTROMETRY: FRAGMENTATION_MODE {modo}\n\
         AC$MASS_SPECTROMETRY: COLLISION_ENERGY 35 (nominal)\nMS$FOCUSED_ION: PRECURSOR_M/Z 136.0757\nMS$FOCUSED_ION: PRECURSOR_TYPE {addotto}\n\
         PK$PEAK: m/z int. rel.int.\n  94.0651 1000 999\n  136.0757 60 60\n//\n"
    )
}

fn scrivi(nome: &str, testo: &str) -> String {
    let d = std::env::temp_dir().join(format!("mzlab_msn_rules_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    let p = d.join(nome);
    std::fs::write(&p, testo).unwrap();
    p.to_str().unwrap().to_string()
}

fn righe(o: &crate::Outcome) -> Vec<mzlab_core::json::Value> {
    String::from_utf8_lossy(&o.stdout)
        .lines()
        .map(|l| parse(l.as_bytes()).unwrap())
        .collect()
}

#[test]
fn a_record_gives_a_line_of_numbers_with_the_rule_that_explains_the_loss() {
    let f = scrivi(
        "a.txt",
        &registro("SYN-A", "LC-ESI-QFT", "HCD", "[M+H]+", "CC(=O)Nc1ccccc1"),
    );
    let o = run(&["msn-rules", &f, "--regole"]);
    assert_eq!(o.code, 0, "{}", o.stderr);
    let l = &righe(&o)[0];
    assert_eq!(l.get("strato").unwrap().str(), Some("hcd"));
    let v = l.get("v").unwrap();
    let n = |k: &str| v.get(k).unwrap().f64().unwrap();
    // one fragment peak (the precursor is not counted), explained by the sub-formula and by a rule of the structure
    assert_eq!((n("n"), n("a"), n("c")), (1.0, 1.0, 1.0));
    assert_eq!(n("ic"), 100.0);
    assert_eq!(
        v.get("r").unwrap().get("loss_c2h2o_acetyl").unwrap().f64(),
        Some(1.0)
    );
}

#[test]
fn the_stratum_comes_from_the_instrument_and_the_mode() {
    let casi = [
        ("LC-ESI-ITFT", "CID", "cid_trappola"),
        ("LC-ESI-ITFT", "HCD", "hcd"),
        ("LC-ESI-QTOF", "CID", "qtof"),
        ("LC-ESI-QQ", "CID", "fascio"),
    ];
    for (k, (strumento, modo, atteso)) in casi.iter().enumerate() {
        let f = scrivi(
            &format!("s{k}.txt"),
            &registro(
                &format!("SYN-S{k}"),
                strumento,
                modo,
                "[M+H]+",
                "CC(=O)Nc1ccccc1",
            ),
        );
        let l = righe(&run(&["msn-rules", &f])).remove(0);
        assert_eq!(
            l.get("strato").unwrap().str(),
            Some(*atteso),
            "{strumento} {modo}"
        );
        assert!(
            l.get("v").unwrap().get("c").is_none(),
            "without --regole only (a)"
        );
    }
}

#[test]
fn a_record_that_cannot_be_judged_says_why() {
    let senza = registro("SYN-N", "LC-ESI-QFT", "HCD", "[M+H]+", "N/A");
    let aduct = registro(
        "SYN-D",
        "LC-ESI-QFT",
        "HCD",
        "[M+H-H2O]+",
        "CC(=O)Nc1ccccc1",
    );
    let modo = registro("SYN-M", "LC-ESI-QFT", "ETD", "[M+H]+", "CC(=O)Nc1ccccc1");
    for (t, motivo) in [(senza, "smiles"), (aduct, "ione"), (modo, "attivazione")] {
        let f = scrivi("x.txt", &t);
        let l = righe(&run(&["msn-rules", &f])).remove(0);
        assert_eq!(l.get("saltato").unwrap().str(), Some(motivo));
    }
}

#[test]
fn the_decoys_explain_less_and_the_same_seed_gives_the_same_lines() {
    let a = scrivi(
        "d1.txt",
        &registro("SYN-D1", "LC-ESI-QFT", "HCD", "[M+H]+", "CC(=O)Nc1ccccc1"),
    );
    let b = scrivi(
        "d2.txt",
        &registro("SYN-D2", "LC-ESI-QFT", "HCD", "[M+H]+", "CC(=O)Nc1ccccc1O")
            .replace("C8H9NO", "C8H9NO2"),
    );
    let args = ["msn-rules", &a, "--regole", "--decoy", "5"];
    let (x, y) = (run(&args), run(&args));
    assert_eq!(x.stdout, y.stdout);
    let l = &righe(&run(&["msn-rules", &a, "--regole", "--decoy", "5"]))[0];
    // the losses shifted by 13.7 mDa fall out of every window: nothing is explained
    let dl = l.get("dl").unwrap();
    assert_eq!(dl.get("a").unwrap().f64(), Some(0.0));
    assert_eq!(dl.get("c").unwrap().f64(), Some(0.0));
    // a lone record has no other spectrum of its nominal precursor
    assert!(l.get("dp").is_none());
    // two records with the same nominal precursor but another structure give each other the spectrum
    let l2 = righe(&run(&["msn-rules", &a, &b, "--regole", "--decoy", "5"]));
    assert!(l2.iter().all(|l| l.get("dp").is_some()));
    // the same compound at another energy (same neutral formula) is not a decoy
    let a2 = scrivi(
        "d3.txt",
        &registro("SYN-D3", "LC-ESI-QFT", "HCD", "[M+H]+", "CC(=O)Nc1ccccc1"),
    );
    let l3 = righe(&run(&["msn-rules", &a, &a2, "--regole", "--decoy", "5"]));
    assert!(l3.iter().all(|l| l.get("dp").is_none()));
}

#[test]
fn a_nominal_mass_record_is_counted_apart() {
    let m = |a: f64| {
        format!("{{\"n\":2,\"it\":100,\"a\":1,\"ia\":{a},\"b\":0,\"ib\":0,\"c\":0,\"ic\":0,\"r\":{{}}}}")
    };
    let t = format!(
        "{{\"id\":\"N\",\"strato\":\"fascio\",\"pol\":1,\"v\":{}}}\n{{\"id\":\"H\",\"strato\":\"fascio\",\"pol\":1,\"v\":{}}}\n",
        m(20.0),
        m(90.0)
    );
    let s = parse(somma(&t).unwrap().as_bytes()).unwrap();
    let st = &s.get("strati").unwrap().arr()[0];
    assert_eq!(st.get("nominali").unwrap().f64(), Some(1.0));
    assert_eq!(st.get("registri").unwrap().f64(), Some(1.0));
}

#[test]
fn the_sum_applies_the_thresholds_of_the_protocol() {
    let m = |c: f64, r: &str| {
        format!("{{\"n\":2,\"it\":100,\"a\":2,\"ia\":100,\"b\":0,\"ib\":0,\"c\":1,\"ic\":{c},\"r\":{{{r}}}}}")
    };
    let mut t = String::new();
    // 25 records: the rule `loss_c2h2o_acetyl` fires in the true ones; the spectrum decoy explains nothing, the losses decoy fires it in 10 of them
    for k in 0..25 {
        let dl = if k < 10 {
            m(60.0, "\"loss_c2h2o_acetyl\":1")
        } else {
            m(0.0, "")
        };
        t += &format!(
            "{{\"id\":\"R{k}\",\"strato\":\"hcd\",\"pol\":1,\"v\":{},\"dp\":{},\"dl\":{dl}}}\n",
            m(80.0, "\"loss_c2h2o_acetyl\":1"),
            m(0.0, ""),
        );
    }
    t += "{\"id\":\"X\",\"saltato\":\"smiles\"}\n";
    let s = parse(somma(&t).unwrap().as_bytes()).unwrap();
    assert_eq!(
        s.get("saltati").unwrap().get("smiles").unwrap().f64(),
        Some(1.0)
    );
    let st = &s.get("strati").unwrap().arr()[0];
    assert_eq!(st.get("registri").unwrap().f64(), Some(25.0));
    assert_eq!(
        st.get("copertura").unwrap().get("c").unwrap().f64(),
        Some(0.8)
    );
    let dl = st.get("decoy").unwrap().get("dl").unwrap();
    assert_eq!(
        dl.get("differenza").unwrap().f64(),
        Some(0.8 - 0.6 * 10.0 / 25.0)
    );
    // dp and dl are both far enough (0.8 and 0.56 above) from the true ones
    assert_eq!(st.get("soglia_copertura").unwrap().bool(), Some(true));
    let r = &st.get("regole").unwrap().arr()[0];
    assert_eq!(r.get("vero").unwrap().f64(), Some(25.0));
    // 10 of 25 = 0.4 > 0.30 with 25 applications (at least 20): priority 3
    assert_eq!(r.get("tasso").unwrap().f64(), Some(0.4));
    assert_eq!(r.get("esito").unwrap().str(), Some("priorita_3"));
}

#[test]
fn a_rule_with_fewer_than_twenty_applications_is_not_measured() {
    let m = "{\"n\":1,\"it\":10,\"a\":1,\"ia\":10,\"b\":0,\"ib\":0,\"c\":1,\"ic\":10,\"r\":{\"loss_c2h2o_acetyl\":1}}";
    let t =
        format!("{{\"id\":\"R\",\"strato\":\"cid_trappola\",\"pol\":-1,\"v\":{m},\"dl\":{m}}}\n");
    let s = parse(somma(&t).unwrap().as_bytes()).unwrap();
    let st = &s.get("strati").unwrap().arr()[0];
    assert_eq!(
        st.get("regole").unwrap().arr()[0]
            .get("esito")
            .unwrap()
            .str(),
        Some("non_misurata")
    );
    assert_eq!(st.get("soglia_copertura").unwrap().bool(), Some(false));
}
