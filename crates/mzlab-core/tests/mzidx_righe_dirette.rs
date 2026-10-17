//! The scan table and the `.mzidx` of a reader built without the mzML text (`Rows::lettore`, one decoding for the table and pass 1) equal the old way:
//! the mzML `lettori_mzml::scrivi` writes, read back by `scantable` and indexed by `build_mzml`. A made-up reader (`lettore_finto`) carries every optional
//! field: precursor with and without a value, every polarity, empty scans, equal m/z in several scans, NaN and out-of-range m/z, 64-bit intensities,
//! arrays of unequal length, extras that name accessions the rows read. Header variants too (analyzer components, `MS:1000768`, names that need the text).
#![allow(clippy::indexing_slicing)]
#![allow(clippy::disallowed_methods)]

mod lettore_finto;

use lettore_finto::Finto;
use mzlab_core::json::{self, Value};
use mzlab_core::lettori::Lettore;
use mzlab_core::mzmlwrite::Header;
use mzlab_core::{lettori_mzml, mzidx, upload};

const BIG: usize = mzidx::DEFAULT_BUDGET;

fn u64_at(b: &[u8], at: usize) -> u64 {
    u64::from_le_bytes(b[at..at + 8].try_into().unwrap())
}

fn json_at(b: &[u8], off: usize, len: usize) -> Value {
    json::parse(&b[u64_at(b, off) as usize..][..u64_at(b, len) as usize]).unwrap()
}

fn without(v: &Value, skip: &[&str]) -> Vec<(String, Value)> {
    match v {
        Value::Obj(o) => o
            .iter()
            .filter(|(k, _)| !skip.contains(&k.as_str()))
            .cloned()
            .collect(),
        _ => panic!("not an object"),
    }
}

/// Index through the mzML (the oracle) and the two ways straight from the reader, with several budgets.
fn prova(label: &str, l: Finto) {
    prova_con(label, l, true);
}

/// `con_mzml`: false when the mzML of the reader is not valid (then only the two ways straight from the reader are compared).
fn prova_con(label: &str, mut l: Finto, con_mzml: bool) {
    let name = format!("{label}.mzML");
    let a = con_mzml.then(|| {
        let mzml = lettori_mzml::scrivi(&mut l, 0, Vec::new(), &mut |_, _| {}).unwrap();
        let pa = upload::prepare(&mut mzml.clone(), mzml.len() as u64, &name).unwrap();
        let (idx, _) = mzidx::build_mzml_to_vec(&pa.table, &mzml, BIG)
            .unwrap_or_else(|e| panic!("{label}: mzML path {e:?}"));
        mzml_con_scratch(label, &pa.table, &mzml, &idx);
        idx
    });

    let pb = upload::prepare_lettore(&mut l, 0, &[], &name).unwrap();
    let due = |l: &mut Finto, budget: usize| {
        let mut out = std::io::Cursor::new(Vec::new());
        mzidx::build_lettore(&pb.table, l, 0, &mut out, budget).map(|st| (out.into_inner(), st))
    };
    let (b, _) = due(&mut l, BIG).unwrap();
    // the same index in one call, whatever the budget (one pass, or the rounds)
    let mut saw_many = false;
    for budget in [BIG, 1 << 22, 1 << 20, 1 << 18, 1 << 16, 1 << 14, 1 << 12] {
        let mut out = std::io::Cursor::new(Vec::new());
        let mut fasi = Vec::new();
        let Ok((st, sha)) =
            upload::indicizza_lettore(&mut l, 0, &[], &name, &mut out, budget, &mut |f| {
                fasi.push(f)
            })
        else {
            continue;
        };
        assert_eq!(fasi, [3], "{label}");
        assert_eq!(sha, pb.sha256, "{label}");
        saw_many |= st.passes > 1;
        assert!(
            out.into_inner() == b,
            "{label}: the one-call index differs at budget {budget}"
        );
        if let Ok((x, _)) = due(&mut l, budget) {
            assert!(x == b, "{label}: build_lettore differs at budget {budget}");
        }
    }
    if l.indice(0).unwrap().len() > 200 {
        assert!(saw_many, "{label}: no budget forced the rounds");
    }
    con_scratch(label, &mut l, &pb, &name, &b);

    let Some(a) = a else { return };
    // against the mzML: the sections, the rows but for their first two columns, the metadata but for source and version
    let scans_off = u64_at(&a, 32) as usize;
    assert_eq!(u64_at(&b, 32) as usize, scans_off, "{label}: sections");
    assert!(
        a[64..scans_off] == b[64..scans_off],
        "{label}: sections differ"
    );
    let (ra, rb) = (json_at(&a, 32, 40), json_at(&b, 32, 40));
    assert_eq!(ra.arr().len(), rb.arr().len(), "{label}: rows");
    for (k, (x, y)) in ra.arr().iter().zip(rb.arr()).enumerate() {
        assert_eq!(x.arr()[2..], y.arr()[2..], "{label}: row {k}");
    }
    let (ma, mb) = (json_at(&a, 16, 24), json_at(&b, 16, 24));
    assert_eq!(
        without(&ma, &["source", "version"]),
        without(&mb, &["source", "version"]),
        "{label}: metadata"
    );
}

/// The mzML path with a scratch file (`build_mzml_spill`): the same bytes at every budget, the file decoded once.
fn mzml_con_scratch(label: &str, table: &mzidx::ScanTable, mzml: &[u8], idx: &[u8]) {
    for budget in [1 << 20, 1 << 16, 1 << 12] {
        let (mut out, mut sp) = (
            std::io::Cursor::new(Vec::new()),
            std::io::Cursor::new(Vec::new()),
        );
        let Ok(st) = mzidx::build_mzml_spill(
            table,
            std::io::Cursor::new(mzml),
            &mut out,
            budget,
            Some(&mut sp),
        ) else {
            continue;
        };
        assert_eq!(st.passes, 1, "{label}: one decoding with a scratch file");
        assert!(
            out.into_inner() == idx,
            "{label}: mzML index with a scratch file differs at budget {budget}"
        );
    }
}

/// The same index with a scratch file: the peaks that do not fit go to it, the scans are decoded once, the bytes are the same.
fn con_scratch(label: &str, l: &mut Finto, pb: &upload::Prepared, name: &str, b: &[u8]) {
    let mut spilled = false;
    for budget in [BIG, 1 << 22, 1 << 20, 1 << 18, 1 << 16, 1 << 14, 1 << 12] {
        let mut out = std::io::Cursor::new(Vec::new());
        let mut sp = std::io::Cursor::new(Vec::new());
        let Ok((st, sha)) = upload::indicizza_lettore_con(
            l,
            0,
            &mut || Ok(vec![]),
            name,
            &mut out,
            budget,
            Some(&mut sp),
            &mut |_| {},
        ) else {
            continue;
        };
        assert_eq!(sha, pb.sha256, "{label}");
        assert_eq!(st.passes, 1, "{label}: one decoding with a scratch file");
        spilled |= !sp.into_inner().is_empty();
        assert!(
            out.into_inner() == b,
            "{label}: the index with a scratch file differs at budget {budget}"
        );
    }
    if l.indice(0).unwrap().len() > 200 {
        assert!(spilled, "{label}: no budget used the scratch file");
    }
}

fn testata_orbitrap() -> Header {
    Header {
        run: "r".into(),
        sources: vec!["r.raw".into()],
        software: ("mzLab".into(), "1".into()),
        components: vec![
            (
                "source".into(),
                "MS:1000073".into(),
                "electrospray ionization".into(),
            ),
            ("analyzer".into(), "MS:1000484".into(), "orbitrap".into()),
        ],
        source_cv: vec![("MS:1000768".into(), "Thermo nativeID format".into())],
        ..Header::default()
    }
}

#[test]
fn vario_senza_testata() {
    for (n, peaks, seed) in [
        (0, 10, 1),
        (1, 10, 2),
        (7, 5, 3),
        (60, 40, 4),
        (400, 60, 5),
        (1200, 30, 6),
    ] {
        prova(&format!("vario{n}"), Finto::vario(n, peaks, seed));
    }
}

#[test]
fn vario_con_testata_orbitrap() {
    prova(
        "orbitrap",
        Finto::vario(300, 50, 7).con_testata(testata_orbitrap()),
    );
}

#[test]
fn vario_a_bassa_risoluzione() {
    let mut l = Finto::vario(300, 50, 8);
    for s in &mut l.schede {
        s.spettro.filter = s
            .spettro
            .filter
            .replace("FTMS", "TQMS")
            .replace("ITMS", "TQMS");
    }
    prova("lr", l);
}

#[test]
fn testata_con_nomi_che_chiedono_il_testo() {
    let mut h = testata_orbitrap();
    h.components.push((
        "analyzer".into(),
        "MS:1000079".into(),
        "quad\"rupole".into(),
    ));
    prova_con("nomi", Finto::vario(120, 20, 9).con_testata(h), false);
}

#[test]
fn un_solo_campione_di_una_polarita() {
    let mut l = Finto::vario(150, 30, 10);
    for s in &mut l.schede {
        s.spettro.polarity = 1;
    }
    prova("positivo", l);
}

#[test]
fn array_di_lunghezza_diversa() {
    prova_con(
        "lunghezze",
        Finto::vario(200, 30, 11).con_lunghezze_diverse(),
        false,
    );
}

#[test]
fn uguali_dopo_l_arrotondamento_tra_due_bucket() {
    // every MS1 scan holds 100.0 and every other one 99.9999999 too (bucket 99 in 64 bits, 100 in 32): with the scratch file the partitions of the two
    // buckets are read one after the other, and the equal 32-bit m/z must still come in scan order
    let mut l = Finto::vario(600, 10, 13);
    for (k, s) in l.schede.iter_mut().enumerate() {
        let sp = &mut s.spettro;
        sp.wide = None;
        if sp.level == 1 {
            sp.mz = if k % 2 == 1 {
                vec![99.5, 99.999_999_9, 100.0, 100.5]
            } else {
                vec![99.5, 100.0, 100.5]
            };
            sp.intensity = (0..sp.mz.len()).map(|j| (k * 10 + j) as f32).collect();
        }
    }
    let mzml = lettori_mzml::scrivi(&mut l, 0, Vec::new(), &mut |_, _| {}).unwrap();
    let pa = upload::prepare(&mut mzml.clone(), mzml.len() as u64, "uguali.mzML").unwrap();
    assert!(pa.table.hr1, "the MS1 sections hold f32");
    let (idx, _) = mzidx::build_mzml_to_vec(&pa.table, &mzml, BIG).unwrap();
    mzml_con_scratch("uguali", &pa.table, &mzml, &idx);
}
