#![cfg(feature = "thermo")]
//! The synthetic Orbitrap v66 writer (`examples/thermo_sintetico.rs`, the stand-in for the speed benches of high-resolution `.raw`) gives a file the reader opens:
//! the scan count, the MS1/MS2 cycle, profile MS1 with centroids at the m/z it wrote, MS2 with a precursor. Small file in a temporary folder, removed afterwards.
//! A pinned hash of every decoded scan (centroids and profile) is the gate of any change to the decoder: it must give the same values.
use mzlab_core::lettori::{FileSet, Limiti, Membro};
use mzlab_core::mzml::Spectrum;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::process::Command;

/// The built example next to the test binary (`cargo test` builds it), else `cargo run`.
fn genera(out: &Path, args: &[&str]) {
    let ex = std::env::current_exe()
        .ok()
        .and_then(|e| Some(e.parent()?.parent()?.join("examples/thermo_sintetico")))
        .filter(|p| p.is_file());
    let st = match ex {
        Some(p) => Command::new(p).arg(out).args(args).status(),
        None => Command::new(env!("CARGO"))
            .args([
                "run",
                "--release",
                "-q",
                "-p",
                "mzlab-lettori",
                "--features",
                "thermo",
                "--example",
                "thermo_sintetico",
                "--",
            ])
            .arg(out)
            .args(args)
            .status(),
    };
    assert!(st.is_ok_and(|s| s.success()), "thermo_sintetico failed");
}

fn cartella(nome: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!(
        "mzlab_thermo_sintetico_{}_{nome}",
        std::process::id()
    ));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn the_synthetic_orbitrap_file_opens_with_its_cycle() {
    let d = cartella("ciclo");
    let f = d.join("hr.raw");
    genera(
        &f,
        &[
            "--scans",
            "66",
            "--peaks",
            "40",
            "--ms2",
            "10",
            "--ms2-peaks",
            "12",
            "--trailer",
            "30",
            "--seed",
            "7",
        ],
    );
    let b = std::fs::read(&f).unwrap();
    let _ = std::fs::remove_dir_all(&d);
    let set = FileSet {
        membri: vec![Membro {
            nome: "hr.raw".into(),
            dim: b.len() as u64,
            dati: Box::new(b),
            sha1: None,
        }],
    };
    let mut l = mzlab_lettori::apri(set, &Limiti::sito()).expect("open");
    let ix = l.index(0).expect("index").to_vec();
    assert_eq!(ix.len(), 66);
    for (k, v) in ix.iter().enumerate() {
        assert_eq!(v.livello, if k % 11 == 0 { 1 } else { 2 }, "scan {k}");
        assert_eq!(v.polarita, 1);
        assert!(v.filtro.starts_with("FTMS"), "{}", v.filtro);
        assert_eq!(v.precursore.is_some(), v.livello == 2, "scan {k}");
    }
    for k in [0usize, 11, 55] {
        let s = l.scan(0, k).expect("ms1");
        assert_eq!(s.level, 1);
        assert_eq!(s.mz.len(), 40, "the centroid list of the packet");
        assert!(s.mz.windows(2).all(|w| w[0] < w[1]));
        assert!(s.mz.iter().all(|&m| (100.0..=1500.0).contains(&m)));
        let p = l
            .scan_profile(0, k)
            .expect("profile")
            .expect("an MS1 profile");
        assert!(p.mz.len() > s.mz.len());
    }
    let s = l.scan(0, 1).expect("ms2");
    assert_eq!(s.level, 2);
    assert_eq!(s.mz.len(), 12);
    assert!(s.precursor.is_some_and(|p| p > 150.0));
}

#[test]
fn the_same_seed_writes_the_same_bytes() {
    let d = cartella("seme");
    let (a, b) = (d.join("a.raw"), d.join("b.raw"));
    for f in [&a, &b] {
        genera(
            f,
            &[
                "--scans", "24", "--peaks", "30", "--ms2", "3", "--seed", "3",
            ],
        );
    }
    let (x, y) = (std::fs::read(&a).unwrap(), std::fs::read(&b).unwrap());
    let _ = std::fs::remove_dir_all(&d);
    assert_eq!(x, y);
}

fn fold(h: &mut Sha256, s: &Spectrum) {
    h.update((s.mz.len() as u64).to_le_bytes());
    s.mz.iter().for_each(|m| h.update(m.to_le_bytes()));
    s.intensity.iter().for_each(|y| h.update(y.to_le_bytes()));
    if let Some(w) = &s.wide {
        w.iter().for_each(|y| h.update(y.to_le_bytes()));
    }
    h.update([s.level]);
    h.update(s.rt.to_le_bytes());
    h.update(s.precursor.unwrap_or(-1.0).to_le_bytes());
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[test]
fn the_decoded_scans_of_a_profile_file_keep_their_pinned_hash() {
    let d = cartella("impronta");
    let f = d.join("p.raw");
    let args = [
        "--scans", "120", "--peaks", "300", "--bins", "16", "--ms2", "5", "--seed", "11",
    ];
    genera(&f, &args);
    let b = std::fs::read(&f).unwrap();
    let _ = std::fs::remove_dir_all(&d);
    let file = hex(&Sha256::digest(&b));
    let set = FileSet {
        membri: vec![Membro {
            nome: "p.raw".into(),
            dim: b.len() as u64,
            dati: Box::new(b),
            sha1: None,
        }],
    };
    let mut l = mzlab_lettori::apri(set, &Limiti::sito()).expect("open");
    let n = l.index(0).expect("index").len();
    let mut h = Sha256::new();
    for k in 0..n {
        fold(&mut h, &l.scan(0, k).expect("scan"));
        if let Some(p) = l.scan_profile(0, k).expect("profile") {
            fold(&mut h, &p);
        }
    }
    let got = hex(&h.finalize());
    // the method document: the trailer passes (the field ranges and the series) are in it
    let method = hex(&Sha256::digest(l.method(0).expect("method").as_bytes()));
    eprintln!("file {file} decoded {got} method {method}");
    assert_eq!(method, METHOD_SHA256, "the method document changed");
    assert_eq!(
        file, FILE_SHA256,
        "the writer changed: a new pin needs a new reason"
    );
    assert_eq!(got, DECODED_SHA256, "the decoder gives other values");
}

const FILE_SHA256: &str = "e2d77e897f6c86a56a78d0f149acaf0026904a250e9d5e23f49b554c5482099f";
const METHOD_SHA256: &str = "abad0fdfdb1af251aaf3a5debee54f29fa4712bb7eb772a0d6bf032a0bdced8d";
const DECODED_SHA256: &str = "e261d343d99bf77b0137844eb261914460852073efd3f7a4efac23b241666ddb";

#[test]
fn the_metadata_from_the_known_trailer_fields_is_the_metadata_from_the_whole_record() {
    // `scan_metadata` (the index at open) decodes only the trailer fields `ScanParams` reads: every scan must come out as from the whole record
    let d = cartella("campi");
    let f = d.join("hr.raw");
    genera(
        &f,
        &[
            "--scans", "44", "--peaks", "20", "--ms2", "10", "--seed", "3",
        ],
    );
    let b = std::fs::read(&f).unwrap();
    let _ = std::fs::remove_dir_all(&d);
    let mut c = std::io::Cursor::new(b);
    let raw = opentfraw::RawFileReader::open(&mut c).expect("open");
    let first = raw.run_header.sample_info.first_scan_number;
    for k in 0..raw.num_scans {
        let whole = raw.scan_params(&mut c, first + k).expect("trailer");
        let known = raw.scan_params_known(&mut c, first + k).expect("trailer");
        assert!(known.record().values.len() < whole.record().values.len());
        let a = opentfraw::mzml::scan_metadata_with(&raw, k, Some(whole));
        let b = opentfraw::mzml::scan_metadata(&raw, &mut c, k);
        assert_eq!(format!("{a:?}"), format!("{b:?}"), "scan {k}");
        assert!(b.is_some_and(|m| k % 11 == 0 || m.precursor.is_some()));
    }
}

/// `/api/dda_window` of a DDA `.raw` written with `args`, for the queries `qs(last RT, scans)`: (page, app and CLI, index alone) per query. The page reads
/// the index and source of the mzML the reader writes, the app and CLI those of the reader.
fn dda_windows(
    name: &str,
    args: &[&str],
    qs: impl Fn(f64, &[mzlab_core::idxread::Scan]) -> Vec<String>,
) -> Vec<(String, String, String)> {
    use mzlab_core::idxread::Index;
    use mzlab_core::routes::{Query, Routes};
    use mzlab_core::sorgente::SorgenteLettore;
    use mzlab_core::{lettori_mzml, mzidx, upload};
    let d = cartella(name);
    let f = d.join("dda.raw");
    genera(&f, args);
    let b = std::fs::read(&f).unwrap();
    let _ = std::fs::remove_dir_all(&d);
    let mut set = FileSet {
        membri: vec![Membro {
            nome: "dda.raw".into(),
            dim: b.len() as u64,
            dati: Box::new(b),
            sha1: None,
        }],
    };
    let membri = upload::impronte(&mut set).unwrap();
    let mut l = mzlab_lettori::apri(set, &Limiti::desktop()).expect("open");
    let mzml = lettori_mzml::scrivi(&mut *l, 0, Vec::new(), &mut |_, _| {}).unwrap();
    let pa = upload::prepare(&mut mzml.clone(), mzml.len() as u64, "dda.mzML").unwrap();
    let (ia, _) = mzidx::build_mzml_to_vec(&pa.table, &mzml, mzidx::DEFAULT_BUDGET).unwrap();
    let pb = upload::prepare_lettore(&mut *l, 0, &membri, "dda.mzML").unwrap();
    let mut ib = std::io::Cursor::new(Vec::new());
    mzidx::build_lettore(&pb.table, &mut *l, 0, &mut ib, mzidx::DEFAULT_BUDGET).unwrap();
    let ib = ib.into_inner();
    let mut ra = Routes::new(Index::open(ia.clone(), ia.len() as u64).unwrap());
    let mut rb = Routes::new(Index::open(ib.clone(), ib.len() as u64).unwrap());
    let hi = ra.idx.scans.iter().map(|s| s.rt).fold(0.0, f64::max);
    qs(hi, &ra.idx.scans.clone())
        .iter()
        .map(|q| {
            let q = Query::parse(q);
            let a = ra
                .mzml_route("/api/dda_window", &q, &mut mzml.clone())
                .unwrap()
                .unwrap();
            let c = rb
                .mzml_route(
                    "/api/dda_window",
                    &q,
                    &mut SorgenteLettore { l: &mut *l, c: 0 },
                )
                .unwrap()
                .unwrap();
            let plain = ra.handle("/api/dda_window", &q).unwrap().unwrap();
            (a, c, plain)
        })
        .collect()
}

/// A reader that gives no precursor intensity: the page and the app and CLI give the same bytes, the intensities filled from the parent survey scans.
#[test]
fn the_dda_window_of_a_raw_fills_its_precursor_intensities_alike() {
    let args = [
        "--scans", "60", "--peaks", "200", "--ms2", "5", "--seed", "7", "--dda",
    ];
    let qs = |hi: f64, _: &[mzlab_core::idxread::Scan]| {
        vec![
            format!("r0=0&r1={hi}"),
            format!("r0={}&r1={}", hi / 3.0, hi / 2.0),
            format!("r0=0&r1={hi}&hr=0"),
        ]
    };
    for (a, c, plain) in dda_windows("ddaw", &args, qs) {
        assert_eq!(a, c);
        assert!(!a.contains("null"), "a precursor without intensity: {a}");
        assert!(!a.contains("pintCut"), "{a}");
        assert!(
            plain.contains("null"),
            "the reader gives intensities: {plain}"
        );
    }
}

/// Past 400 parent survey scans the rest stay null and the answer says so (`pintCut`, the notice of the «N MS²» list), alike on every surface;
/// a narrower window is filled whole and the index alone never claims the cut.
#[test]
fn the_dda_window_of_a_raw_says_where_it_stops_filling() {
    let args = [
        "--scans", "830", "--peaks", "20", "--ms2", "1", "--seed", "11", "--dda",
    ];
    let qs = |hi: f64, _: &[mzlab_core::idxread::Scan]| {
        vec![format!("r0=0&r1={hi}"), format!("r0=0&r1={}", hi / 4.0)]
    };
    let r = dda_windows("ddacut", &args, qs);
    let (a, c, plain) = &r[0];
    assert_eq!(a, c);
    assert!(a.contains(r#""pintCut":400"#) && a.contains("null"), "{a}");
    assert!(!plain.contains("pintCut"), "{plain}");
    let (a, c, _) = &r[1];
    assert_eq!(a, c);
    assert!(!a.contains("pintCut") && !a.contains("null"), "{a}");
}

/// One precursor over the whole run (`mz`, «Segui questo ione» and the precursor list of dda.js): only its parents are read, so the run that
/// stops filling at 400 survey scans gives every intensity of that precursor, alike on every surface, and only its MS2.
#[test]
fn the_dda_window_of_one_precursor_reads_only_its_parents() {
    let args = [
        "--scans", "830", "--peaks", "20", "--ms2", "1", "--seed", "11", "--dda",
    ];
    let qs = |hi: f64, scans: &[mzlab_core::idxread::Scan]| {
        let m = scans
            .iter()
            .find_map(|s| s.precursor.filter(|_| s.level > 1))
            .unwrap();
        vec![
            format!("r0=0&r1={hi}&mz={m}"),
            format!("r0=0&r1={hi}&mz={m}&tol=0.001"),
            format!("r0=0&r1={hi}&mz=1e9"),
        ]
    };
    let r = dda_windows("ddamz", &args, qs);
    for (a, c, plain) in &r[..2] {
        assert_eq!(a, c);
        assert!(a.contains(r#""groups":[{"prec":"#), "{a}");
        assert!(!a.contains("pintCut") && !a.contains("null"), "{a}");
        assert!(plain.contains("null"), "{plain}");
        assert_eq!(a.matches("\"prec\"").count(), 1, "{a}");
    }
    assert!(r[2].0.ends_with(r#""groups":[]}"#), "{}", r[2].0);
}
