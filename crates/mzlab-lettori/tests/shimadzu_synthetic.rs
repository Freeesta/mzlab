#![cfg(feature = "shimadzu")]
//! The Shimadzu module on containers built here (invented numbers): scans, SIM, MRM as SRM chromatograms, polarity, the method without identity, and the refusals.
mod common;

use common::shimadzu::*;
use mzlab_core::mzml::ChromKind;
use mzlab_core::readers::{FileSet, Limits, Member, Reader};

fn open(name: &str, b: Vec<u8>) -> mzlab_core::error::Result<Box<dyn Reader>> {
    mzlab_lettori::open_bare(
        FileSet {
            membri: vec![Member {
                nome: name.into(),
                dim: b.len() as u64,
                dati: Box::new(b),
                sha1: None,
            }],
        },
        &Limits::sito(),
    )
}

#[test]
fn qgd_scans_sim_and_tic() {
    let mut l = open("a.qgd", qgd(6)).unwrap();
    let ix = l.index(0).unwrap().to_vec();
    assert_eq!(ix.len(), 11);
    assert_eq!(ix[0].id, "scan=1");
    assert!(ix[0].filtro.contains("EI scan"));
    assert!(ix[10].filtro.contains("EI SIM"), "{}", ix[10].filtro);
    assert!((ix[2].rt - 2000.0 / 60_000.0).abs() < 1e-12);
    let s = l.scan(0, 1).unwrap();
    assert_eq!(s.mz, vec![50.0, 51.0, 55.0, 60.05]);
    assert_eq!(s.intensity, vec![101.0, 7.0, 3001.0, 1.0]);
    assert_eq!(s.intensity.iter().sum::<f32>() as f64, ix[1].tic);
    let c = l.chromatograms(0).unwrap();
    assert_eq!(c[0].id, "TIC");
    assert_eq!(l.chromatogram(0, "TIC").unwrap().time.len(), 11);
}

#[test]
fn qgd_mrm_gives_srm_chromatograms() {
    let mut l = open("m.qgd", qgd_mrm(4)).unwrap();
    assert!(l.index(0).unwrap().is_empty());
    let c = l.chromatograms(0).unwrap();
    let srm: Vec<_> = c.iter().filter(|v| v.tipo == ChromKind::Srm).collect();
    assert_eq!(srm.len(), 2);
    let a = l.chromatogram(0, &srm[0].id).unwrap();
    assert_eq!((a.q1, a.q3), (Some(200.0), Some(100.0)));
    assert_eq!(a.intensity, vec![10.0, 11.0, 12.0, 13.0]);
}

#[test]
fn lcd2020_scans_and_sim_by_event() {
    let mut l = open("a.lcd", lcd2020(3)).unwrap();
    let ix = l.index(0).unwrap().to_vec();
    assert_eq!(ix.len(), 6);
    assert!(ix[0].filtro.starts_with("Q1 event 1"));
    assert!(ix[5].filtro.starts_with("Q1 event 2"));
    let s = l.scan(0, 0).unwrap();
    assert_eq!(s.mz.len(), 8);
    assert_eq!(s.mz[0], 100.0);
    assert_eq!(s.intensity.iter().sum::<f32>() as f64, ix[0].tic);
}

#[test]
fn lcd_tlm_srm_chromatograms() {
    let mut l = open("q.lcd", lcd_tlm(5)).unwrap();
    let c = l.chromatograms(0).unwrap();
    let srm: Vec<_> = c.iter().filter(|v| v.tipo == ChromKind::Srm).collect();
    assert_eq!(srm.len(), 2);
    assert!(srm[0]
        .id
        .starts_with("SRM SIC Q1=176.1 Q3=159.1 sample=1 period=1 experiment=1 transition=0"));
    let a = l.chromatogram(0, &srm[0].id).unwrap();
    assert_eq!(a.intensity, vec![50.0, 51.0, 52.0, 53.0, 54.0]);
}

#[test]
fn the_method_keeps_the_operator_apart() {
    let mut l = open("a.qgd", qgd(3)).unwrap();
    let m = l.method(0).unwrap();
    assert!(m.contains("GCMS-QP2010"), "{m}");
    assert!(
        m.contains("operatorXyz"),
        "the reader's document holds the operator"
    );
    let h = l.mzml_header(0).unwrap().unwrap();
    let doc = format!("{:?}", h.run_params);
    assert!(
        !doc.contains("operatorXyz") && !doc.contains("personQ"),
        "{doc}"
    );
    assert!(doc.contains("2021-03-04T05:06:07"), "{doc}");
}

#[test]
fn qtof_and_pda_only_are_refused_with_a_key() {
    let e = open("t.lcd", lcd_qtof()).err().expect("refused");
    assert_eq!(e.key, "err.file.shimadzu");
    let e = open("p.lcd", lcd_pda_only()).err().expect("refused");
    assert_eq!(e.key, "err.file.shimadzu");
}

#[test]
fn the_extension_alone_is_not_enough() {
    assert!(open("x.qgd", vec![0u8; 4000]).is_err());
}

#[test]
fn the_sample_name_has_no_folder() {
    for p in ["dir/sub/a.qgd", "dir\\sub\\a.qgd"] {
        let l = open(p, qgd(6)).unwrap();
        assert_eq!(l.samples()[0].nome, "a.qgd", "{p}");
    }
}

#[test]
fn tlm_entries_that_share_one_record_are_not_inflated_again() {
    // 2 000 index entries all naming the record of the first: a record is inflated once per entry, so a 16 MB zlib record would cost 2 000 x 16 MB
    let good = lcd_tlm(3);
    let mut l = open("q.lcd", good).unwrap();
    let id = l
        .chromatograms(0)
        .unwrap()
        .into_iter()
        .find(|v| v.tipo == ChromKind::Srm)
        .unwrap()
        .id;
    assert_eq!(
        l.chromatogram(0, &id).unwrap().intensity,
        vec![50.0, 51.0, 52.0]
    );
    let mut l = open("q.lcd", lcd_tlm_shared(2000)).unwrap();
    let srm: Vec<_> = l
        .chromatograms(0)
        .unwrap()
        .into_iter()
        .filter(|v| v.tipo == ChromKind::Srm)
        .collect();
    // only the entry whose record is its own is read
    assert!(srm
        .iter()
        .all(|c| l.chromatogram(0, &c.id).unwrap().intensity.len() <= 1));
}
