use super::*;

fn ms1(rt: f64) -> Scan {
    Scan {
        level: 1,
        rt,
        ..Default::default()
    }
}

/// An MSn scan with the filter path `stages` (m/z, CE), all CID.
fn msn(rt: f64, stages: &[(f64, f64)]) -> Scan {
    Scan {
        level: stages.len() as u8 + 1,
        rt,
        precursor: stages.last().map(|s| s.0),
        path: Some(
            stages
                .iter()
                .map(|(m, ce)| (*m, "CID".into(), *ce))
                .collect(),
        ),
        ..Default::default()
    }
}

/// An infusion from Tune: one path at a time, each held for 20 scans, an MS1 survey between the blocks.
fn tune_blocks() -> Vec<Scan> {
    let paths: [&[(f64, f64)]; 4] = [
        &[(317.2, 25.0)],
        &[(317.2, 25.0), (261.1, 30.0)],
        &[(317.2, 25.0), (299.2, 30.0)],
        &[(317.2, 25.0), (261.1, 30.0), (188.0, 35.0)],
    ];
    let mut v = vec![];
    let mut rt = 0.0;
    for p in paths {
        v.push(ms1(rt));
        for _ in 0..20 {
            rt += 0.01;
            v.push(msn(rt, p));
        }
    }
    v
}

/// A targeted LC run (PRM): 10 fixed precursors cycled after each MS1, 30 cycles.
fn prm() -> Vec<Scan> {
    let mut v = vec![];
    for c in 0..30 {
        let rt = c as f64 * 0.05;
        v.push(ms1(rt));
        for p in 0..10 {
            v.push(msn(
                rt + 0.001 * (p + 1) as f64,
                &[(150.0 + 20.0 * p as f64, 26.0)],
            ));
        }
    }
    v
}

#[test]
fn tune_blocks_are_an_infusion() {
    let got = infusione(&tune_blocks(), None, "sample.mzML").unwrap();
    assert_eq!(
        got,
        Infusione {
            motivo: "blocchi",
            livelli: 4,
            nodi: 4
        }
    );
}

#[test]
fn cycled_fixed_precursors_need_a_hint() {
    let v = prm();
    assert_eq!(infusione(&v, None, "run_t10.mzML"), None);
    assert_eq!(infusione(&v, Some(true), "run_t10.mzML"), None);
    let m = infusione(&v, Some(false), "run_t10.mzML").unwrap();
    assert_eq!((m.motivo, m.livelli, m.nodi), ("metodo_senza_lc", 2, 10));
    let n = infusione(&v, None, "dir/std_Infusion_MSn.mzML").unwrap();
    assert_eq!(n.motivo, "nome");
}

#[test]
fn dda_is_never_an_infusion() {
    let mut v = vec![];
    for c in 0..200 {
        let rt = c as f64 * 0.02;
        v.push(ms1(rt));
        for p in 0..3 {
            v.push(msn(
                rt + 0.001,
                &[(100.0 + c as f64 * 1.7 + p as f64 * 0.31, 30.0)],
            ));
        }
    }
    assert_eq!(infusione(&v, Some(false), "direct_infusion.mzML"), None);
}

#[test]
fn constant_rt_or_one_scan() {
    let mut v = tune_blocks();
    v.iter_mut().for_each(|s| s.rt = 1.5);
    assert_eq!(
        infusione(&v, Some(true), "x.mzML").unwrap().motivo,
        "rt_costante"
    );
    let one = infusione(&[ms1(3.0)], None, "x.mzML").unwrap();
    assert_eq!((one.motivo, one.livelli, one.nodi), ("rt_costante", 1, 0));
    let run: Vec<Scan> = (0..50).map(|i| ms1(i as f64 * 0.1)).collect();
    assert_eq!(infusione(&run, Some(false), "infusion.mzML"), None);
    assert_eq!(infusione(&[], None, "infusion.mzML"), None);
}

#[test]
fn energies_split_keys_but_not_nodes() {
    let mut v = vec![ms1(0.0)];
    for (i, ce) in [20.0, 30.0, 40.0].into_iter().enumerate() {
        for j in 0..12 {
            v.push(msn(1.0 + i as f64 + j as f64 * 0.01, &[(301.1, ce)]));
        }
    }
    let got = infusione(&v, None, "a.mzML").unwrap();
    assert_eq!((got.motivo, got.nodi), ("blocchi", 1));
}
