//! The channels of synthetic files: the lists below are written by hand from the structure of each file, not read back from the code.

use super::*;

pub(super) fn scan(level: u8, rt: f64, stages: &[(f64, f64)]) -> Scan {
    Scan {
        level,
        rt,
        precursor: stages.last().map(|s| s.0),
        path: (!stages.is_empty()).then(|| {
            stages
                .iter()
                .map(|(m, ce)| (*m, "CID".to_string(), *ce))
                .collect()
        }),
        ..Default::default()
    }
}

/// (id, family, level, n, parent) of every channel.
pub(super) fn riassunto(c: &Canali) -> Vec<(String, &'static str, u8, u32, Option<u32>)> {
    c.v.iter()
        .map(|x| (x.id.clone(), x.famiglia.nome(), x.livello, x.n, x.genitore))
        .collect()
}

pub(super) fn voce(
    id: &str,
    f: &'static str,
    l: u8,
    n: u32,
    g: Option<u32>,
) -> (String, &'static str, u8, u32, Option<u32>) {
    (id.to_string(), f, l, n, g)
}

pub(super) fn da(scans: &[Scan]) -> Canali {
    Canali::da(scans, &[], &Facts::default(), "x.mzML")
}

#[test]
fn full_scan_file() {
    let scans: Vec<Scan> = (0..30).map(|i| scan(1, i as f64 * 0.1, &[])).collect();
    let c = da(&scans);
    assert_eq!(
        riassunto(&c),
        [
            voce("tic", "tic", 0, 30, None),
            voce("bpc", "bpc", 0, 30, Some(0)),
            voce("ms:ms1", "ms", 1, 30, Some(0))
        ]
    );
    assert_eq!(
        c.tipo,
        Tipo {
            proposto: "lc-full",
            motivo: "ms1_only"
        }
    );
    assert_eq!(c.v[2].etichetta, "MS¹");
    assert_eq!(c.v[2].omologo, "ms|1|||||");
    assert!(c.v[2].spettri && !c.v[0].spettri);
}

#[test]
fn targeted_ms2_in_lc_has_one_channel_per_precursor() {
    // MS1, then MS2 of 222.1 and of 300.2 at 35 eV, every cycle
    let mut scans = vec![];
    for i in 0..20 {
        let rt = i as f64 * 0.1;
        scans.push(scan(1, rt, &[]));
        scans.push(scan(2, rt + 0.01, &[(222.1, 35.0)]));
        scans.push(scan(2, rt + 0.02, &[(300.2, 35.0)]));
    }
    let c = da(&scans);
    assert_eq!(
        riassunto(&c),
        [
            voce("tic", "tic", 0, 60, None),
            voce("bpc", "bpc", 0, 60, Some(0)),
            voce("ms:ms1", "ms", 1, 20, Some(0)),
            voce("ms:ms2 222 @cid", "ms", 2, 20, Some(2)),
            voce("ms:ms2 300 @cid", "ms", 2, 20, Some(2)),
        ]
    );
    assert_eq!(c.tipo.proposto, "lc-ms2-mirato");
    assert_eq!(c.v[3].etichetta, "MS² 222 @CID35");
    assert_eq!(c.v[3].omologo, "ms|2|222|cid|35||");
    // the partition: scans of the channel in file order, 4 bytes per scan
    assert_eq!(
        c.scansioni(3),
        (0..20).map(|i| 3 * i + 1).collect::<Vec<u32>>()
    );
    assert_eq!(
        c.scansioni(4),
        (0..20).map(|i| 3 * i + 2).collect::<Vec<u32>>()
    );
    assert_eq!(c.scansioni(0), (0..60).collect::<Vec<u32>>());
    assert_eq!(c.scan_ids.len(), 60);
}

#[test]
fn msn_in_blocks_is_a_tree_of_channels() {
    // an infusion: MS1, then 20 scans of MS2 317, MS3 317>261 and MS3 317>299 held one after the other
    let paths: [&[(f64, f64)]; 3] = [
        &[(317.2, 25.0)],
        &[(317.2, 25.0), (261.1, 30.0)],
        &[(317.2, 25.0), (299.2, 30.0)],
    ];
    let mut scans = vec![];
    let mut rt = 0.0;
    for p in paths {
        scans.push(scan(1, rt, &[]));
        for _ in 0..20 {
            rt += 0.01;
            scans.push(scan(p.len() as u8 + 1, rt, p));
        }
    }
    let c = da(&scans);
    assert_eq!(
        riassunto(&c),
        [
            voce("tic", "tic", 0, 63, None),
            voce("bpc", "bpc", 0, 63, Some(0)),
            voce("ms:ms1", "ms", 1, 3, Some(0)),
            voce("ms:ms2 317 @cid", "ms", 2, 20, Some(2)),
            voce("ms:ms3 317 > 261 @cid30", "ms", 3, 20, Some(3)),
            voce("ms:ms3 317 > 299 @cid30", "ms", 3, 20, Some(3)),
        ]
    );
    assert_eq!(
        c.tipo,
        Tipo {
            proposto: "infusione-msn",
            motivo: "blocks"
        }
    );
    assert_eq!(c.v[4].etichetta, "MS³ 317→261 @CID30");
    assert_eq!(c.v[4].omologo, "ms|3|317>261|cid|30||");
}

#[test]
fn many_precursors_make_one_dda_channel() {
    // 70 distinct precursors at level 2, each MS2 hanging from the cycle's MS1; one of them has an MS3
    let mut scans = vec![];
    for i in 0..70 {
        let rt = i as f64 * 0.05;
        scans.push(scan(1, rt, &[]));
        scans.push(scan(2, rt + 0.01, &[(200.0 + i as f64 * 3.0, 30.0)]));
    }
    scans.push(scan(3, 4.0, &[(200.0, 30.0), (150.0, 30.0)]));
    let c = da(&scans);
    assert_eq!(
        riassunto(&c),
        [
            voce("tic", "tic", 0, 141, None),
            voce("bpc", "bpc", 0, 141, Some(0)),
            voce("ms:ms1", "ms", 1, 70, Some(0)),
            voce("dd:2", "dd", 2, 70, Some(0)),
            voce("ms:ms3 200 > 150 @cid30", "ms", 3, 1, Some(3)),
        ]
    );
    assert_eq!(c.v[3].sigla, Some("DDA"));
    assert_eq!(c.v[3].etichetta, "MS² DDA");
    assert_eq!(
        c.tipo,
        Tipo {
            proposto: "lc-dda",
            motivo: "many_precursors"
        }
    );
    assert_eq!(c.scan_ids.len(), 141);
}

#[test]
fn two_polarities_with_the_same_key_are_two_channels() {
    let mut scans = vec![];
    for i in 0..10 {
        for pol in [1i8, -1] {
            scans.push(Scan {
                polarity: pol,
                ..scan(1, i as f64 * 0.1, &[])
            });
        }
    }
    let c = da(&scans);
    let ids: Vec<&str> = c.v.iter().map(|x| x.id.as_str()).collect();
    assert_eq!(ids, ["tic", "bpc", "ms:ms1#+", "ms:ms1#-"]);
    assert_eq!((c.v[2].pol, c.v[3].pol), (1, -1));
}

pub(super) fn srm(index: usize, q1: f64, q3: f64, ce: f64, name: &str) -> Chrom {
    Chrom {
        index,
        id: format!("SRM SIC Q1={q1} Q3={q3}"),
        kind: "srm".into(),
        q1: Some(q1),
        q3: Some(q3),
        ce: Some(ce),
        name: name.into(),
        n: Some(400),
        ..Default::default()
    }
}

#[test]
fn mrm_file_has_a_summed_tic_and_one_channel_per_transition() {
    let chroms = vec![
        srm(0, 230.0, 162.0, 25.0, "bentazone 1"),
        srm(1, 230.0, 190.0, 18.0, ""),
    ];
    let facts = Facts {
        srm_rt: Some((0.0, 20.0)),
        ..Default::default()
    };
    let c = Canali::da(&[], &chroms, &facts, "m.mzML");
    assert_eq!(
        riassunto(&c),
        [
            voce("tic", "tic", 0, 400, None),
            voce("srm:230>162@25", "srm", 2, 400, Some(0)),
            voce("srm:230>190@18", "srm", 2, 400, Some(0)),
        ]
    );
    assert_eq!(c.v[0].sorgente, Sorgente::Somma);
    assert_eq!(c.v[1].sorgente, Sorgente::Cromatogramma(0));
    assert_eq!(c.v[1].etichetta, "MRM 230→162 @25 «bentazone 1»");
    assert_eq!(c.v[1].omologo, "srm|2|230.0>162.0||25||");
    assert!(!c.v[1].spettri);
    assert_eq!(
        c.tipo,
        Tipo {
            proposto: "lc-mrm",
            motivo: "srm_only"
        }
    );
    // the TIC of the maker is the TIC of the file
    let mut con_tic = chroms.clone();
    con_tic.push(Chrom {
        index: 2,
        id: "TIC".into(),
        kind: "tic".into(),
        ..Default::default()
    });
    let c = Canali::da(&[], &con_tic, &facts, "m.mzML");
    assert_eq!(c.v[0].sorgente, Sorgente::Cromatogramma(2));
}

#[test]
fn mrm_with_product_ion_scans_is_mixed() {
    let scans: Vec<Scan> = (0..10)
        .map(|i| scan(2, i as f64 * 0.1, &[(364.1 + (i % 2) as f64, 30.0)]))
        .collect();
    let c = Canali::da(
        &scans,
        &[srm(1, 364.1, 194.1, 20.0, "Quant")],
        &Facts::default(),
        "c.mzML",
    );
    assert_eq!(c.tipo.proposto, "misto");
    let ids: Vec<&str> = c.v.iter().map(|x| x.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "tic",
            "bpc",
            "ms:ms2 364 @cid",
            "ms:ms2 365 @cid",
            "srm:364.1>194.1@20"
        ]
    );
}

#[test]
fn a_file_with_only_a_uv_trace_has_no_channel_yet() {
    let c = Canali::da(
        &[],
        &[],
        &Facts {
            pda: true,
            ..Default::default()
        },
        "u.mzML",
    );
    assert!(c.v.is_empty());
    assert_eq!(c.tipo.proposto, "solo-uv");
}

#[test]
fn memory_beyond_the_scan_table_is_at_most_eight_bytes_per_scan() {
    let n = 100_000usize;
    let scans: Vec<Scan> = (0..n)
        .map(|i| match i % 5 {
            0 => scan(1, i as f64 * 1e-3, &[]),
            k => scan(2, i as f64 * 1e-3, &[(200.0 + k as f64 * 10.0, 30.0)]),
        })
        .collect();
    let c = da(&scans);
    assert_eq!(c.v.len(), 2 + 1 + 4);
    assert!(c.byte() <= 8 * n, "{} bytes for {n} scans", c.byte());
}

/// A scan of a `.wiff` as the index holds it: no filter, the id of the experiment, the precursor of the method (if any).
fn scan_wiff(cycle: usize, exp: usize, level: u8, prec: Option<f64>) -> Scan {
    Scan {
        level,
        rt: cycle as f64 * 0.05,
        precursor: prec,
        native: format!("sample=1 period=1 cycle={cycle} experiment={exp}"),
        ..Default::default()
    }
}

#[test]
fn wiff_ems_and_epi_at_fixed_precursors_are_ms_channels_with_a_sigla() {
    // EMS, then two product-ion experiments with their own precursor
    let mut scans = vec![];
    for c in 1..=10 {
        scans.push(scan_wiff(c, 1, 1, None));
        scans.push(scan_wiff(c, 2, 2, Some(230.1)));
        scans.push(scan_wiff(c, 3, 2, Some(305.0)));
    }
    let c = da(&scans);
    let ms: Vec<_> = c.v.iter().filter(|x| x.famiglia == Famiglia::Ms).collect();
    assert_eq!(ms.len(), 3);
    assert_eq!((ms[0].sigla, ms[0].livello), (Some("EMS"), 1));
    assert_eq!(ms[0].etichetta, "MS¹ EMS");
    assert!(ms[1..].iter().all(|x| x.sigla == Some("EPI") && x.n == 10));
    assert!(c.v.iter().all(|x| x.famiglia != Famiglia::Dd));
}

#[test]
fn wiff_ida_dependent_scans_are_one_dd_channel() {
    // EMS and three dependent EPI slots with no precursor in the method; a slot is empty (no scan) in some cycles
    let mut scans = vec![];
    for c in 1..=10 {
        scans.push(scan_wiff(c, 1, 1, None));
        for e in 2..=4 {
            if (c + e) % 3 != 0 {
                scans.push(scan_wiff(c, e, 2, None));
            }
        }
    }
    let n_epi = scans.iter().filter(|s| s.level == 2).count() as u32;
    let c = da(&scans);
    let dd: Vec<_> = c.v.iter().filter(|x| x.famiglia == Famiglia::Dd).collect();
    assert_eq!(dd.len(), 1);
    assert_eq!(
        (dd[0].id.as_str(), dd[0].sigla, dd[0].n),
        ("dd:2:EPI", Some("EPI"), n_epi)
    );
    assert_eq!(dd[0].genitore, Some(0));
    assert_eq!(dd[0].omologo, "dd|2|EPI");
    assert!(dd[0].spettri);
    assert_eq!(c.v.iter().filter(|x| x.famiglia == Famiglia::Ms).count(), 1);
    assert_eq!(c.tipo.proposto, "lc-dda");
}

#[test]
fn a_scan_without_the_wiff_id_has_no_wiff_sigla() {
    let c = da(&[scan(1, 0.0, &[]), scan(2, 0.1, &[(222.0, 30.0)])]);
    assert!(c.v.iter().skip(2).all(|x| x.sigla.is_none()));
}

#[test]
fn mrm_label_is_the_method_name_else_the_transition() {
    // the ids msconvert and the engine write: the name rides in `name=`, the label reads it back
    let ids = [
        "SRM SIC Q1=230 Q3=162 sample=1 period=1 experiment=1 transition=0 ce=25 dwell=50 name=bentazone 1",
        "SRM SIC Q1=230 Q3=190 sample=1 period=1 experiment=1 transition=1 ce=18 dwell=50",
    ];
    let chroms: Vec<Chrom> = ids
        .iter()
        .enumerate()
        .map(|(i, id)| {
            let (q1, q3, ce, name) = crate::mzmlhead::srm_id(id);
            Chrom {
                index: i,
                id: id.to_string(),
                kind: "srm".into(),
                q1,
                q3,
                ce,
                name,
                n: Some(100),
                ..Default::default()
            }
        })
        .collect();
    let c = Canali::da(&[], &chroms, &Facts::default(), "m.mzML");
    assert_eq!(c.v[1].etichetta, "MRM 230→162 @25 «bentazone 1»");
    assert_eq!(c.v[2].etichetta, "MRM 230→190 @18");
}
