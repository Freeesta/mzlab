//! `.mzlib` v2: a v1 file still opens, the retention index and the library info are written and read back, and the parsers fill them in.
mod common;
use common::*;
use mzlab_lib::build::{build_in_memory, BuildOpts, Builder};
use mzlab_lib::libinfo::{inchikey14, Declared, Kind, Tech};
use mzlab_lib::parse::{parse_all, Format};
use mzlab_lib::read::{Library, CACHE_BYTES};
use mzlab_lib::search::{identity, IdentityOpts, Query, Tol};
use mzlab_lib::sha256::{hex, sha256};

/// A file written by the v1 builder from four synthetic spectra (precursors 200.1, 250.2, 300.3, 350.4; the last one negative).
const V1: &[u8] = include_bytes!("data/v1_small.mzlib");

#[test]
fn a_v1_file_is_read_as_before() {
    let lib = Library::open(V1, CACHE_BYTES).unwrap();
    assert_eq!(lib.header.version, 1);
    assert_eq!(lib.n_spectra(), 4);
    lib.verify().unwrap();
    assert_eq!(*lib.info(), Default::default());
    assert!(lib.inchikeys().unwrap().iter().all(|k| *k == 0));
    // negative first, then by precursor
    let names: Vec<String> = (0..4).map(|i| lib.meta(i).unwrap().name).collect();
    assert_eq!(
        names,
        [
            "V1 spectrum 3",
            "V1 spectrum 0",
            "V1 spectrum 1",
            "V1 spectrum 2"
        ]
    );
    for i in 0..4 {
        let r = lib.rec(i).unwrap();
        assert_eq!(r.ri_val(), None);
        assert!(r.rt_min().is_some());
    }
    let (mz, _) = lib.peaks(1).unwrap();
    assert_eq!(mz, [60.5, 90.25, 120.125, 182.1]);
    let q = Query::new(200.1, 1, &mz, &[0.25, 1.0, 0.5, 2.0]).unwrap();
    let o = IdentityOpts {
        prec_tol: Tol::Da(0.5),
        frag_da: 0.05,
        pol: 1,
        top: 3,
    };
    let hits = identity(&lib, &q, &o).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(lib.meta(hits[0].index).unwrap().name, "V1 spectrum 0");
}

fn gc_set() -> Vec<mzlab_lib::spec::Spectrum> {
    let mut set = synth_set(5, 30, 12);
    for (i, s) in set.iter_mut().enumerate() {
        s.ri = (i % 5 != 0).then_some(1000.0 + i as f64 * 7.5);
        s.ei = i % 5 == 0;
        s.meta.instrument = "GC-EI-TOF".into();
    }
    set
}

#[test]
fn retention_index_info_and_inchikeys_are_written_and_read_back() {
    let set = gc_set();
    let mut b = Builder::new(Vec::<u8>::new(), BuildOpts::default());
    b.set_source("mb", "2025.1", "abc123");
    for s in &set {
        b.add(s).unwrap();
    }
    let mut out = Vec::new();
    b.finish(&mut out).unwrap();
    let lib = Library::open(&out[..], CACHE_BYTES).unwrap();
    assert_eq!(lib.header.version, 2);
    assert_eq!(lib.header.min_reader, 1);
    lib.verify().unwrap();
    let info = lib.info();
    assert_eq!(info.tech, Some(Tech::GcEi));
    assert_eq!(info.kind, Some(Kind::Experimental));
    assert_eq!(info.pos + info.neg + info.unknown_pol, 30);
    assert_eq!(info.analyzers.get("ei"), Some(&30));
    assert_eq!(
        (
            info.source.as_str(),
            info.version.as_str(),
            info.sha256.as_str()
        ),
        ("mb", "2025.1", "abc123")
    );
    // every spectrum keeps its own RI and key whatever the order of the table
    let keys = lib.inchikeys().unwrap();
    let mut with_ri = 0;
    for i in 0..lib.n_spectra() {
        let m = lib.meta(i).unwrap();
        let n: usize = m.name.trim_start_matches("Synthetic ").parse().unwrap();
        assert_eq!(keys[i as usize], inchikey14(&m.inchikey));
        let r = lib.rec(i).unwrap();
        match r.ri_val() {
            Some(v) => {
                with_ri += 1;
                assert_eq!(v, (1000.0 + n as f64 * 7.5) as f32 as f64);
            }
            None => assert!(n.is_multiple_of(5)),
        }
    }
    assert_eq!(with_ri, 24);
    assert_ne!(lib.fingerprint(), [0u8; 32]);
}

#[test]
fn the_kind_and_technique_can_be_declared() {
    let set = synth_set(6, 10, 8);
    let opts = BuildOpts {
        declared: Declared {
            kind: Some(Kind::InSilico),
            tech: Some(Tech::Msn),
        },
        ..BuildOpts::default()
    };
    let (b, _) = build_in_memory(&set, opts).unwrap();
    let lib = Library::open(&b[..], CACHE_BYTES).unwrap();
    assert_eq!(
        (lib.info().kind, lib.info().tech),
        (Some(Kind::InSilico), Some(Tech::Msn))
    );
    // without a declaration the detection speaks: Orbitrap, LC, measured
    let (b, _) = build_in_memory(&set, BuildOpts::default()).unwrap();
    let lib = Library::open(&b[..], CACHE_BYTES).unwrap();
    assert_eq!(
        (lib.info().kind, lib.info().tech),
        (Some(Kind::Experimental), Some(Tech::LcMsms))
    );
    assert_eq!(lib.info().analyzers.get("orbitrap"), Some(&10));
    // in silico by name
    let mut named = set.clone();
    for s in named.iter_mut() {
        s.meta.name = format!("LipidBlast {}", s.meta.name);
    }
    let (b, _) = build_in_memory(&named, BuildOpts::default()).unwrap();
    let lib = Library::open(&b[..], CACHE_BYTES).unwrap();
    assert_eq!(lib.info().kind, Some(Kind::InSilico));
}

#[test]
fn the_parsers_read_the_retention_index_and_the_ionisation() {
    let msp = b"Name: A\nPrecursorMZ: 150.1\nRetention_index: 1234.5\nIonization: EI\nNum Peaks: 2\n50 10\n100 20\n\n\
Name: B\nPrecursorMZ: 160.1\nKovats: 1500\nSpectrum_type: MS2\nNum Peaks: 2\n50 10\n100 20\n\n\
Name: C\nPrecursorMZ: 170.1\nRI: abc\nNum Peaks: 1\n50 10\n\n";
    let (v, _) = parse_all(Format::Msp, msp);
    assert_eq!(v.len(), 3);
    assert_eq!((v[0].ri, v[0].ei), (Some(1234.5), true));
    assert_eq!((v[1].ri, v[1].ei), (Some(1500.0), false));
    assert_eq!((v[2].ri, v[2].ei), (None, false));
    let mb = b"ACCESSION: X1\nRECORD_TITLE: t\nAC$CHROMATOGRAPHY: KOVATS_RI 1700\nAC$MASS_SPECTROMETRY: IONIZATION EI\nMS$FOCUSED_ION: PRECURSOR_M/Z 200.5\nPK$PEAK: m/z int. rel.int.\n  50 100 999\n  60 50 500\n//\n";
    let (v, _) = parse_all(Format::MassBank, mb);
    assert_eq!(v.len(), 1);
    assert_eq!((v[0].ri, v[0].ei), (Some(1700.0), true));
}

#[test]
fn sha256_of_the_header_changes_with_the_content() {
    let a = synth_set(7, 20, 8);
    let mut b = a.clone();
    b[3].mz[1] += 0.5;
    let la = build_in_memory(&a, BuildOpts::default()).unwrap().0;
    let lb = build_in_memory(&b, BuildOpts::default()).unwrap().0;
    let fa = Library::open(&la[..], CACHE_BYTES).unwrap().fingerprint();
    let fb = Library::open(&lb[..], CACHE_BYTES).unwrap().fingerprint();
    let fa2 = Library::open(&la[..], CACHE_BYTES).unwrap().fingerprint();
    assert_eq!(fa, fa2);
    assert_ne!(fa, fb);
    assert_eq!(hex(&sha256(&[b"abc"])).len(), 64);
}
